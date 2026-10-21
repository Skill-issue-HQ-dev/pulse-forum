//! Tests for web::api through the real router: health, version, status, and `POST /api/entry` (new, retry, taken,
//! honeypot, closed, bad JSON, 413, every 422 code, characters not bytes, headers, a database failure).

mod common;

use axum::http::StatusCode;
use common::{app, valid_entry, App, Reply, KEY};
use pulse_forum::core::record::Row;
use pulse_forum::io::store;
use serde_json::{json, Value};

fn stored(app: &App) -> Vec<Row> {
    store::list(&app.state.db.lock().unwrap()).unwrap()
}

fn refusal(code: &str) -> Value {
    json!({ "ok": false, "error": code })
}

#[tokio::test]
async fn health_version_and_status_answer_json() {
    let app = app();
    let r = app.get("/healthz").await;
    assert_eq!((r.status, r.json()), (StatusCode::OK, json!({ "ok": true })));
    assert!(r.header("content-type").starts_with("application/json"));
    assert_eq!(app.get("/version").await.json(), json!({ "version": env!("CARGO_PKG_VERSION") }));
    assert_eq!(app.get("/api/status").await.json(), json!({ "ok": true, "open": true, "count": 0 }));
    app.submit(1).await;
    app.submit(2).await;
    assert_eq!(app.get("/api/status").await.json(), json!({ "ok": true, "open": true, "count": 2 }));
    app.set_open(false).await;
    assert_eq!(app.get("/api/status").await.json(), json!({ "ok": true, "open": false, "count": 2 }));
}

#[tokio::test]
async fn a_valid_entry_is_stored_trimmed_and_answered_new() {
    let app = app();
    let mut body = valid_entry(1);
    body["first"] = json!("  Testa ");
    body["story"] = json!("\n Stāsts ar atstarpēm abos galos, vismaz divdesmit rakstzīmes.  ");
    body["share"] = json!(true);
    let r = app.entry(&body).await;
    assert_eq!((r.status, r.json()), (StatusCode::OK, json!({ "ok": true, "status": "new" })));
    let rows = stored(&app);
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].id, rows[0].first.as_str(), rows[0].story.as_str()), (1, "Testa", "Stāsts ar atstarpēm abos galos, vismaz divdesmit rakstzīmes."));
    assert!(rows[0].share && !rows[0].excluded && rows[0].won.is_none());
    assert!(rows[0].at.ends_with('Z') && rows[0].at.len() == 20, "RFC 3339 UTC in whole seconds: {}", rows[0].at);
}

#[tokio::test]
async fn a_resend_is_retry_and_a_second_phone_with_the_same_email_is_taken() {
    let app = app();
    assert_eq!(app.entry(&valid_entry(1)).await.json()["status"], "new");
    assert_eq!(app.entry(&valid_entry(1)).await.json(), json!({ "ok": true, "status": "retry" }));
    let mut other = valid_entry(2);
    other["email"] = json!(" TEST1@Example.com ");
    other["story"] = json!("Cits stāsts, kas nedrīkst aizstāt pirmo ierakstu.");
    assert_eq!(app.entry(&other).await.json(), json!({ "ok": true, "status": "taken" }));
    let rows = stored(&app);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].story, valid_entry(1)["story"].as_str().unwrap(), "the first entry is kept unchanged");
    assert_eq!(rows[0].email, "test1@example.com");
}

#[tokio::test]
async fn the_body_is_json_whatever_the_content_type() {
    let app = app();
    for (n, content_type) in [(1u32, Some("text/plain;charset=UTF-8")), (2, Some("application/json")), (3, Some("application/x-www-form-urlencoded")), (4, None)] {
        let headers: Vec<(&str, &str)> = content_type.map(|c| ("content-type", c)).into_iter().collect();
        let r = app.send(common::request("POST", "/api/entry", &headers, valid_entry(n).to_string().into_bytes())).await;
        assert_eq!((r.status, r.json()["status"].as_str()), (StatusCode::OK, Some("new")), "{content_type:?}");
    }
    assert_eq!(stored(&app).len(), 4);
}

#[tokio::test]
async fn the_honeypot_answers_new_and_stores_nothing() {
    let app = app();
    let mut bot = valid_entry(1);
    bot["website"] = json!("http://spam.example");
    let r = app.entry(&bot).await;
    assert_eq!((r.status, r.json()), (StatusCode::OK, json!({ "ok": true, "status": "new" })));
    // Even when the rest is rubbish, and even when entries are closed: the honeypot is checked first.
    assert_eq!(app.entry(&json!({ "website": "x", "story": 7 })).await.status, StatusCode::BAD_REQUEST, "wrong types are still a parse error");
    assert_eq!(app.entry(&json!({ "website": "x" })).await.json()["status"], "new");
    app.set_open(false).await;
    assert_eq!(app.entry(&bot).await.json()["status"], "new");
    assert!(stored(&app).is_empty());
}

#[tokio::test]
async fn closed_is_423_after_the_honeypot_and_before_the_rules() {
    let app = app();
    app.set_open(false).await;
    let r = app.entry(&valid_entry(1)).await;
    assert_eq!((r.status, r.json()), (StatusCode::LOCKED, refusal("closed")));
    let mut invalid = valid_entry(2);
    invalid["consent"] = json!(false);
    assert_eq!(app.entry(&invalid).await.status, StatusCode::LOCKED, "closed beats the rules");
    assert_eq!(app.entry_raw(b"not json".to_vec()).await.status, StatusCode::BAD_REQUEST, "parsing comes first");
    assert!(stored(&app).is_empty());
    app.set_open(true).await;
    assert_eq!(app.entry(&valid_entry(1)).await.json()["status"], "new");
}

#[tokio::test]
async fn not_json_or_wrong_field_types_is_400_json_and_stores_nothing() {
    let app = app();
    let bodies: [&[u8]; 12] = [b"not json", b"", b"{", b"[]", b"\"text\"", b"null", b"42", b"{\"consent\":\"yes\"}", b"{\"story\":5}", b"{\"token\":1}", b"{\"share\":\"no\"}", b"\xff\xfe\x00"];
    for body in bodies {
        let r = app.entry_raw(body.to_vec()).await;
        assert_eq!((r.status, r.json()), (StatusCode::BAD_REQUEST, refusal("json")), "{:?}", String::from_utf8_lossy(body));
    }
    assert!(stored(&app).is_empty());
}

#[tokio::test]
async fn a_body_over_16_kib_is_413_and_exactly_16_kib_is_read() {
    let app = app();
    let mut body = valid_entry(1).to_string().into_bytes();
    body.resize(16 * 1024, b' ');
    let r = app.entry_raw(body.clone()).await;
    assert_eq!((r.status, r.json()["status"].as_str()), (StatusCode::OK, Some("new")), "16 KiB exactly is allowed");
    body.resize(16 * 1024 + 1, b' ');
    let r = app.entry_raw(body).await;
    assert_eq!((r.status, r.json()), (StatusCode::PAYLOAD_TOO_LARGE, refusal("too_large")));
    let r = app.entry_raw(vec![b'x'; 1024 * 1024]).await;
    assert_eq!(r.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(stored(&app).len(), 1);
}

#[tokio::test]
async fn every_422_code_is_answered_over_http_and_stores_nothing() {
    let app = app();
    let cases: [(&str, &str, Value); 10] = [
        ("token", "token", json!("ABCDEF0123456789abcdef0123456789")),
        ("lang", "lang", json!("de")),
        ("category", "category", json!("holiday")),
        ("story_short", "story", json!("too short")),
        ("story_long", "story", json!("x".repeat(1501))),
        ("missing", "phone", json!("")),
        ("too_long", "company", json!("c".repeat(121))),
        ("email", "email", json!("nobody@nowhere")),
        ("phone", "phone", json!("12-34")),
        ("consent", "consent", json!(false)),
    ];
    for (code, field, value) in cases {
        let mut body = valid_entry(1);
        body[field] = value;
        let r = app.entry(&body).await;
        assert_eq!((r.status, r.json()), (StatusCode::UNPROCESSABLE_ENTITY, refusal(code)), "{code}");
    }
    assert!(stored(&app).is_empty());
    let mut no_consent = valid_entry(1);
    no_consent.as_object_mut().unwrap().remove("consent");
    assert_eq!(app.entry(&no_consent).await.json(), refusal("consent"));
}

#[tokio::test]
async fn lengths_are_characters_not_bytes_over_http() {
    let app = app();
    let mut body = valid_entry(1);
    body["story"] = json!("ā".repeat(20));
    body["first"] = json!("ž".repeat(80));
    assert_eq!(app.entry(&body).await.json()["status"], "new");
    let mut short = valid_entry(2);
    short["story"] = json!("ā".repeat(19));
    assert_eq!(app.entry(&short).await.json(), refusal("story_short"));
    let mut long_name = valid_entry(3);
    long_name["last"] = json!("ž".repeat(81));
    assert_eq!(app.entry(&long_name).await.json(), refusal("too_long"));
    let mut emoji = valid_entry(4);
    emoji["story"] = json!("😀".repeat(1500));
    assert_eq!(app.entry(&emoji).await.json()["status"], "new", "1500 emoji are 1500 characters (6 000 bytes)");
}

#[tokio::test]
async fn a_story_survives_unicode_quotes_and_newlines_to_the_admin_list() {
    let app = app();
    let story = "Rīga: \"Ķekava\" & <Jūrmala> 😀\nOtrā rindiņa ar 'pēdiņām'\tun tabulāciju.";
    let mut body = valid_entry(1);
    body["story"] = json!(story);
    assert_eq!(app.entry(&body).await.json()["status"], "new");
    assert_eq!(app.list().await["entries"][0]["story"], story);
}

#[tokio::test]
async fn every_answer_carries_the_security_headers() {
    let app = app();
    let mut invalid = valid_entry(1);
    invalid["lang"] = json!("de");
    let mut answers: Vec<(&str, Reply)> = vec![
        ("200 health", app.get("/healthz").await),
        ("200 entry", app.entry(&valid_entry(2)).await),
        ("400", app.entry_raw(b"nope".to_vec()).await),
        ("404", app.get("/nope").await),
        ("405", app.get("/api/entry").await),
        ("413", app.entry_raw(vec![b' '; 20_000]).await),
        ("422", app.entry(&invalid).await),
        ("401", app.admin_raw("GET", "/api/admin/entries", None, "").await),
        ("204", app.send(common::request("OPTIONS", "/api/entry", &[], vec![])).await),
    ];
    app.set_open(false).await;
    answers.push(("423", app.entry(&valid_entry(3)).await));
    for (what, r) in &answers {
        assert_eq!(r.header("content-security-policy"), "default-src 'none'; frame-ancestors 'none'", "{what}");
        assert_eq!(r.header("x-content-type-options"), "nosniff", "{what}");
        assert_eq!(r.header("referrer-policy"), "no-referrer", "{what}");
        assert_eq!(r.header("vary"), "Origin", "{what}");
        assert_eq!(r.header("access-control-expose-headers"), "content-disposition", "{what}");
    }
    let statuses: Vec<u16> = answers.iter().map(|(_, r)| r.status.as_u16()).collect();
    assert_eq!(statuses, [200, 200, 400, 404, 405, 413, 422, 401, 204, 423]);
}

#[tokio::test]
async fn unknown_paths_and_wrong_methods_are_refused_in_json_or_by_status() {
    let app = app();
    let r = app.get("/nope").await;
    assert_eq!((r.status, r.json()), (StatusCode::NOT_FOUND, refusal("not_found")));
    assert_eq!(app.get("/api/admin").await.status, StatusCode::NOT_FOUND);
    assert_eq!(app.get("/api/entry").await.status, StatusCode::METHOD_NOT_ALLOWED);
    for path in ["/healthz", "/version", "/api/status"] {
        let r = app.send(common::request("POST", path, &[], b"{}".to_vec())).await;
        assert_eq!(r.status, StatusCode::METHOD_NOT_ALLOWED, "{path}");
    }
}

#[tokio::test]
async fn a_database_failure_is_a_plain_500_that_leaks_nothing() {
    let app = app();
    app.state.db.lock().unwrap().execute_batch("DROP TABLE entries").unwrap();
    let r = app.entry(&valid_entry(1)).await;
    assert_eq!((r.status, r.json()), (StatusCode::INTERNAL_SERVER_ERROR, refusal("server")));
    let text = String::from_utf8_lossy(&r.body).to_lowercase();
    for leak in ["sqlite", "table", "entries", "store:"] {
        assert!(!text.contains(leak), "{text}");
    }
    assert_eq!(app.admin_get("/api/admin/entries").await.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(app.get("/healthz").await.status, StatusCode::OK, "the service stays up");
    let _ = KEY;
}

#[tokio::test]
async fn a_resend_after_closing_still_hears_it_is_in() {
    // The answer got lost near the close: the phone resends the stored entry and must not read "closed".
    let app = app();
    assert_eq!(app.entry(&valid_entry(1)).await.json()["status"], "new");
    app.set_open(false).await;
    let r = app.entry(&valid_entry(1)).await;
    assert_eq!((r.status, r.json()["status"].as_str()), (StatusCode::OK, Some("retry")));
    assert_eq!(app.entry(&valid_entry(2)).await.status, StatusCode::LOCKED, "a new entry is still refused");
    assert_eq!(stored(&app).len(), 1);
}

#[tokio::test]
async fn the_same_phone_under_another_email_is_stored_but_out_of_the_draw() {
    let app = app();
    app.entry(&valid_entry(1)).await;
    let mut twin = valid_entry(2);
    twin["phone"] = valid_entry(1)["phone"].clone();
    assert_eq!(app.entry(&twin).await.json()["status"], "new", "the guest hears the usual answer");
    let list = app.list().await;
    assert_eq!((list["entries"][1]["excluded"].as_bool(), list["entries"][1]["dup_of"].as_i64(), list["eligible"].as_i64()), (Some(true), Some(1), Some(1)));
}
