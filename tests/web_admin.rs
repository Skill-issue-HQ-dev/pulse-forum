//! Tests for web::admin: the bearer gate (missing, wrong, shorter, longer, other schemes), the entries JSON,
//! the Excel download, open/close and exclude; the draw and clear routes are in tests/web_draw.rs.

mod common;

use axum::http::StatusCode;
use common::{app, request, KEY};
use pulse_forum::web::check_admin_key;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::io::{Cursor, Read};

fn keys(v: &Value) -> BTreeSet<String> {
    v.as_object().unwrap().keys().cloned().collect()
}

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn sheet(xlsx: &[u8], n: u32) -> String {
    let mut zip = zip::ZipArchive::new(Cursor::new(xlsx.to_vec())).expect("a zip");
    let mut s = String::new();
    zip.by_name(&format!("xl/worksheets/sheet{n}.xml")).expect("the sheet").read_to_string(&mut s).unwrap();
    s
}

/// One request to try: its path, its headers, and the status it must get.
type Case<'a> = (&'a str, Vec<(&'a str, &'a str)>, StatusCode);

const ROUTES: [(&str, &str); 7] = [
    ("GET", "/api/admin/entries"),
    ("GET", "/api/admin/export.xlsx"),
    ("POST", "/api/admin/open"),
    ("POST", "/api/admin/exclude"),
    ("POST", "/api/admin/delete"),
    ("POST", "/api/admin/draw"),
    ("POST", "/api/admin/clear"),
];

#[tokio::test]
async fn every_admin_route_refuses_without_the_exact_key() {
    let app = app();
    app.fill(2).await;
    let (shorter, longer) = (&KEY[..KEY.len() - 1], format!("{KEY}x"));
    let flipped = format!("{}x", &KEY[1..]);
    let body = r#"{"open":false,"id":1,"excluded":true,"confirm":"DZĒST"}"#;
    for (method, path) in ROUTES {
        for key in [None, Some(""), Some("wrong"), Some(shorter), Some(longer.as_str()), Some(flipped.as_str())] {
            let r = app.admin_raw(method, path, key, body).await;
            assert_eq!((r.status, r.json()), (StatusCode::UNAUTHORIZED, json!({ "ok": false, "error": "key" })), "{method} {path} key {key:?}");
            assert_eq!(r.header("cache-control"), "no-store");
        }
    }
    let list = app.list().await;
    assert_eq!((list["open"].as_bool(), list["count"].as_i64(), list["draws"].as_array().unwrap().len()), (Some(true), Some(2), 0), "no refused call changed anything");
}

#[tokio::test]
async fn other_ways_of_sending_the_key_are_refused_and_a_lowercase_scheme_is_fine() {
    let app = app();
    let bearer = format!("Bearer {KEY}");
    let basic = format!("Basic {KEY}");
    let lower = format!("bearer {KEY}");
    let query = format!("/api/admin/entries?key={KEY}");
    let cases: Vec<Case> = vec![
        ("/api/admin/entries", vec![("authorization", KEY)], StatusCode::UNAUTHORIZED),
        ("/api/admin/entries", vec![("authorization", basic.as_str())], StatusCode::UNAUTHORIZED),
        ("/api/admin/entries", vec![("x-api-key", KEY)], StatusCode::UNAUTHORIZED),
        (query.as_str(), vec![], StatusCode::UNAUTHORIZED),
        ("/api/admin/entries", vec![("authorization", "Bearer ")], StatusCode::UNAUTHORIZED),
        ("/api/admin/entries", vec![("authorization", lower.as_str())], StatusCode::OK),
        ("/api/admin/entries", vec![("authorization", bearer.as_str())], StatusCode::OK),
    ];
    for (path, headers, want) in cases {
        let r = app.send(request("GET", path, &headers, vec![])).await;
        assert_eq!(r.status, want, "{path} {headers:?}");
    }
}

#[tokio::test]
async fn the_gate_also_covers_a_wrong_method_but_not_unknown_paths() {
    let app = app();
    assert_eq!(app.admin_raw("GET", "/api/admin/draw", None, "").await.status, StatusCode::UNAUTHORIZED);
    assert_eq!(app.admin_raw("GET", "/api/admin/draw", Some(KEY), "").await.status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(app.admin_raw("POST", "/api/admin/entries", Some(KEY), "{}").await.status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(app.admin_raw("GET", "/api/admin/nope", Some(KEY), "").await.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_entries_json_has_the_contracts_shape() {
    let app = app();
    app.fill(3).await;
    app.admin_post("/api/admin/exclude", &json!({ "id": 2, "excluded": true })).await;
    app.set_open(false).await;
    let drawn = app.admin_post("/api/admin/draw", &json!({})).await.json();
    let (n, winner_id) = (drawn["draw"]["n"].as_i64().unwrap(), drawn["draw"]["winner_id"].as_i64().unwrap());
    let r = app.admin_get("/api/admin/entries").await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(r.header("content-type").starts_with("application/json"));
    let j = r.json();
    assert_eq!(keys(&j), names(&["ok", "open", "count", "eligible", "entries", "draws"]));
    assert_eq!((j["ok"].as_bool(), j["open"].as_bool(), j["count"].as_i64(), j["eligible"].as_i64()), (Some(true), Some(false), Some(3), Some(1)));
    let entries = j["entries"].as_array().unwrap();
    assert_eq!(entries.iter().map(|e| e["id"].as_i64().unwrap()).collect::<Vec<_>>(), [1, 2, 3], "submission order");
    for e in entries {
        assert_eq!(keys(e), names(&["id", "at", "lang", "category", "story", "first", "last", "email", "phone", "company", "share", "excluded", "won", "dup_of"]));
        assert!(e["share"].is_boolean() && e["excluded"].is_boolean() && e["id"].is_i64() && e["at"].as_str().unwrap().ends_with('Z'));
        let phone = format!("+371 2{:07}", e["id"].as_i64().unwrap());
        assert_eq!((e["lang"].as_str(), e["category"].as_str(), e["phone"].as_str()), (Some("lv"), Some("new_contact"), Some(phone.as_str())));
        assert_eq!(e["dup_of"], Value::Null, "every phone differs");
        let want_won = if e["id"] == winner_id { json!(n) } else { Value::Null };
        assert_eq!(e["won"], want_won, "won is the draw number for the winner, null for everyone else");
    }
    assert_eq!(entries[1]["excluded"], true);
    let draws = j["draws"].as_array().unwrap();
    assert_eq!(draws.len(), 1);
    assert_eq!(keys(&draws[0]), names(&["n", "at", "pool", "winner_id", "pool_ids"]));
    assert_eq!((draws[0]["n"].as_i64(), draws[0]["pool"].as_i64(), draws[0]["winner_id"].as_i64()), (Some(1), Some(2), Some(winner_id)));
    assert_ne!(winner_id, 2, "an excluded entry cannot win");
}

#[tokio::test]
async fn an_empty_list_is_still_a_list() {
    let app = app();
    let j = app.list().await;
    assert_eq!(j, json!({ "ok": true, "open": true, "count": 0, "eligible": 0, "entries": [], "draws": [] }));
}

#[tokio::test]
async fn the_export_is_a_three_sheet_workbook_download() {
    let app = app();
    app.fill(3).await;
    app.set_open(false).await;
    app.admin_post("/api/admin/draw", &json!({})).await;
    let r = app.admin_get("/api/admin/export.xlsx").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.header("content-type"), "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet");
    let disposition = r.header("content-disposition");
    let stamp = disposition.strip_prefix("attachment; filename=\"PULSE izloze ").and_then(|s| s.strip_suffix(".xlsx\"")).expect(&disposition);
    assert!(disposition.is_ascii());
    let shape: String = stamp.chars().map(|c| if c.is_ascii_digit() { '9' } else { c }).collect();
    assert_eq!(shape, "9999-99-99 99-99", "the Riga date and time: {stamp}");
    assert!(r.body.starts_with(b"PK"));
    assert_eq!(sheet(&r.body, 1).matches("<row ").count(), 4, "header + 3 entries");
    assert_eq!(sheet(&r.body, 2).matches("<row ").count(), 2, "header + 1 draw");
    assert!(sheet(&r.body, 3).contains("<t>Slēgta</t>"));
    app.set_open(true).await;
    assert!(sheet(&app.admin_get("/api/admin/export.xlsx").await.body, 3).contains("<t>Atvērta</t>"));
}

#[tokio::test]
async fn an_empty_export_still_downloads() {
    let app = app();
    let r = app.admin_get("/api/admin/export.xlsx").await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(sheet(&r.body, 1).matches("<row ").count(), 1);
}

#[tokio::test]
async fn open_and_close_switch_the_public_endpoints() {
    let app = app();
    let r = app.admin_post("/api/admin/open", &json!({ "open": false })).await;
    assert_eq!((r.status, r.json()), (StatusCode::OK, json!({ "ok": true, "open": false })));
    assert_eq!(app.get("/api/status").await.json(), json!({ "ok": true, "open": false, "count": 0 }));
    assert_eq!(app.list().await["open"], false);
    assert_eq!(app.submit(1).await.status, StatusCode::LOCKED);
    let r = app.admin_post("/api/admin/open", &json!({ "open": true })).await;
    assert_eq!(r.json(), json!({ "ok": true, "open": true }));
    assert_eq!(app.submit(1).await.json()["status"], "new");
    assert_eq!(app.admin_post("/api/admin/open", &json!({ "open": true })).await.json(), json!({ "ok": true, "open": true }), "setting the same state twice is fine");
}

#[tokio::test]
async fn exclude_takes_an_entry_out_of_the_draw_and_puts_it_back() {
    let app = app();
    app.fill(3).await;
    let r = app.admin_post("/api/admin/exclude", &json!({ "id": 2, "excluded": true })).await;
    assert_eq!((r.status, r.json()), (StatusCode::OK, json!({ "ok": true })));
    let j = app.list().await;
    assert_eq!((j["entries"][1]["excluded"].as_bool(), j["eligible"].as_i64(), j["count"].as_i64()), (Some(true), Some(2), Some(3)));
    app.admin_post("/api/admin/exclude", &json!({ "id": 2, "excluded": false })).await;
    let j = app.list().await;
    assert_eq!((j["entries"][1]["excluded"].as_bool(), j["eligible"].as_i64()), (Some(false), Some(3)));
}

#[tokio::test]
async fn excluding_an_unknown_entry_is_404_id() {
    let app = app();
    app.fill(1).await;
    for id in [0, 2, 99, -1] {
        let r = app.admin_post("/api/admin/exclude", &json!({ "id": id, "excluded": true })).await;
        assert_eq!((r.status, r.json()), (StatusCode::NOT_FOUND, json!({ "ok": false, "error": "id" })), "id {id}");
    }
    assert_eq!(app.list().await["eligible"], 1);
}

#[tokio::test]
async fn bad_json_on_an_admin_post_is_400_json() {
    let app = app();
    let open_bad = ["", "not json", "{", "[]", "null", "{}", r#"{"open":"yes"}"#, r#"{"open":1}"#, r#"{"open":null}"#];
    let exclude_bad = [
        "", "not json", "[]", "null", "{}", r#"{"id":"1","excluded":true}"#, r#"{"id":1}"#, r#"{"id":1.5,"excluded":true}"#,
        r#"{"excluded":true}"#, r#"{"id":1,"excluded":"yes"}"#, r#"{"id":null,"excluded":true}"#,
    ];
    for (path, bodies) in [("/api/admin/open", &open_bad[..]), ("/api/admin/exclude", &exclude_bad[..])] {
        for body in bodies {
            let r = app.admin_raw("POST", path, Some(KEY), body).await;
            assert_eq!((r.status, r.json()), (StatusCode::BAD_REQUEST, json!({ "ok": false, "error": "json" })), "{path} {body:?}");
        }
    }
    assert_eq!(app.list().await["open"], true, "nothing changed");
}

#[tokio::test]
async fn admin_answers_are_never_cacheable() {
    let app = app();
    app.fill(1).await;
    let posts = [("/api/admin/open", json!({ "open": true })), ("/api/admin/exclude", json!({ "id": 1, "excluded": false }))];
    for path in ["/api/admin/entries", "/api/admin/export.xlsx"] {
        assert_eq!(app.admin_get(path).await.header("cache-control"), "no-store", "{path}");
    }
    for (path, body) in posts {
        assert_eq!(app.admin_post(path, &body).await.header("cache-control"), "no-store", "{path}");
    }
}

#[test]
fn the_admin_key_must_hold_at_least_32_characters() {
    assert!(check_admin_key(&"k".repeat(32)).is_ok());
    assert!(check_admin_key(&"ā".repeat(32)).is_ok(), "characters, not bytes: 16 of them would be 32 bytes");
    assert!(check_admin_key(&"ā".repeat(16)).is_err());
    for short in ["", "k", &"k".repeat(31)] {
        let err = check_admin_key(short).unwrap_err();
        assert!(err.contains("PULSE_ADMIN_KEY") && err.contains("32"), "{err}");
        assert!(short.is_empty() || !err.contains(short), "the message never shows the key");
    }
}

#[tokio::test]
async fn delete_removes_one_entry_but_never_a_drawn_winner() {
    let app = app();
    app.fill(3).await;
    app.set_open(false).await;
    let winner = app.admin_post("/api/admin/draw", &json!({})).await.json()["draw"]["winner_id"].as_i64().unwrap();
    let other = if winner == 1 { 2 } else { 1 };
    let r = app.admin_post("/api/admin/delete", &json!({ "id": winner })).await;
    assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::CONFLICT, Some("winner")));
    let r = app.admin_post("/api/admin/delete", &json!({ "id": other })).await;
    assert_eq!((r.status, r.json()), (StatusCode::OK, json!({ "ok": true, "deleted": other })));
    assert_eq!(app.admin_post("/api/admin/delete", &json!({ "id": other })).await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.list().await["count"], 2);
    assert_eq!(app.admin_raw("POST", "/api/admin/delete", None, r#"{"id":3}"#).await.status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn a_draw_waits_until_entries_are_closed() {
    let app = app();
    app.fill(2).await;
    let r = app.admin_post("/api/admin/draw", &json!({})).await;
    assert_eq!((r.status, r.json()["error"].as_str()), (StatusCode::CONFLICT, Some("open")));
    assert_eq!(app.list().await["draws"], json!([]));
    app.set_open(false).await;
    assert_eq!(app.admin_post("/api/admin/draw", &json!({})).await.status, StatusCode::OK);
}
