//! Tests for the draw and clear routes and for races: one winner per draw among the eligible entries, numbered
//! from 1, 409 when nobody is left; clear needs the exact word and closed entries; concurrent requests stay consistent.

mod common;

use axum::http::StatusCode;
use common::{app, request, send_to, valid_entry, KEY};
use serde_json::{json, Value};
use std::collections::BTreeSet;

const DRAW: &str = "/api/admin/draw";
const CLEAR: &str = "/api/admin/clear";

fn refusal(code: &str) -> Value {
    json!({ "ok": false, "error": code })
}

fn ids(list: &Value, field: &str) -> Vec<i64> {
    list["entries"].as_array().unwrap().iter().filter(|e| !e[field].is_null() && e[field] != false).map(|e| e["id"].as_i64().unwrap()).collect()
}

#[tokio::test]
async fn a_draw_picks_one_eligible_entry_and_answers_with_it() {
    let app = app();
    app.fill(10).await;
    app.set_open(false).await;
    app.admin_post("/api/admin/exclude", &json!({ "id": 4, "excluded": true })).await;
    let r = app.admin_post(DRAW, &json!({})).await;
    assert_eq!(r.status, StatusCode::OK);
    let j = r.json();
    assert_eq!(j["ok"], true);
    let draw = &j["draw"];
    assert_eq!((draw["n"].as_i64(), draw["pool"].as_i64()), (Some(1), Some(9)), "the pool is the eligible count, the first draw is number 1");
    assert!(draw["at"].as_str().unwrap().ends_with('Z'));
    let winner = &j["winner"];
    assert_eq!(winner["id"], draw["winner_id"]);
    assert_ne!(winner["id"], 4, "an excluded entry never wins");
    assert_eq!((winner["won"].as_i64(), winner["excluded"].as_bool()), (Some(1), Some(false)));
    let listed = app.list().await;
    assert_eq!(listed["entries"][winner["id"].as_i64().unwrap() as usize - 1], *winner, "the answer carries the entry exactly as the list shows it");
    assert_eq!(listed["draws"][0], *draw);
    assert_eq!(listed["eligible"], 8);
}

#[tokio::test]
async fn winners_never_repeat_and_the_draw_ends_with_409_empty() {
    let app = app();
    app.fill(6).await;
    app.set_open(false).await;
    app.admin_post("/api/admin/exclude", &json!({ "id": 2, "excluded": true })).await;
    let mut winners = BTreeSet::new();
    for (round, pool) in [(1, 5), (2, 4), (3, 3), (4, 2), (5, 1)] {
        let j = app.admin_post(DRAW, &json!({})).await.json();
        assert_eq!((j["draw"]["n"].as_i64(), j["draw"]["pool"].as_i64()), (Some(round), Some(pool)));
        assert!(winners.insert(j["winner"]["id"].as_i64().unwrap()), "a winner came up twice");
    }
    assert_eq!(winners, BTreeSet::from([1, 3, 4, 5, 6]), "every eligible entry won exactly once");
    let r = app.admin_post(DRAW, &json!({})).await;
    assert_eq!((r.status, r.json()), (StatusCode::CONFLICT, refusal("empty")));
    let list = app.list().await;
    assert_eq!(list["draws"].as_array().unwrap().len(), 5, "the refused draw wrote no row");
    assert_eq!((list["eligible"].as_i64(), list["count"].as_i64()), (Some(0), Some(6)));
}

#[tokio::test]
async fn an_excluded_entry_never_wins_whatever_the_dice_say() {
    for _ in 0..15 {
        let app = app();
        app.fill(5).await;
        app.set_open(false).await;
        for id in 1..=4 {
            app.admin_post("/api/admin/exclude", &json!({ "id": id, "excluded": true })).await;
        }
        assert_eq!(app.admin_post(DRAW, &json!({})).await.json()["winner"]["id"], 5);
    }
}

#[tokio::test]
async fn the_real_randomness_reaches_every_entry() {
    // 3 entries, 60 fresh draws: a stuck source (always the first, always the last) would show; missing one
    // entry by chance has a probability near 1e-10.
    let mut seen = BTreeSet::new();
    for _ in 0..60 {
        let app = app();
        app.fill(3).await;
        app.set_open(false).await;
        seen.insert(app.admin_post(DRAW, &json!({})).await.json()["winner"]["id"].as_i64().unwrap());
    }
    assert_eq!(seen, BTreeSet::from([1, 2, 3]));
}

#[tokio::test]
async fn nobody_eligible_is_409_empty_and_writes_nothing() {
    let app = app();
    app.set_open(false).await;
    let r = app.admin_post(DRAW, &json!({})).await;
    assert_eq!((r.status, r.json()), (StatusCode::CONFLICT, refusal("empty")), "no entries at all");
    app.set_open(true).await;
    app.fill(2).await;
    app.set_open(false).await;
    for id in [1, 2] {
        app.admin_post("/api/admin/exclude", &json!({ "id": id, "excluded": true })).await;
    }
    assert_eq!(app.admin_post(DRAW, &json!({})).await.status, StatusCode::CONFLICT, "all excluded");
    assert!(app.list().await["draws"].as_array().unwrap().is_empty());
    app.admin_post("/api/admin/exclude", &json!({ "id": 2, "excluded": false })).await;
    assert_eq!(app.admin_post(DRAW, &json!({})).await.json()["winner"]["id"], 2, "lifting the exclusion makes it eligible again");
}

#[tokio::test]
async fn a_draw_waits_for_closed_entries_and_wants_a_json_object() {
    let app = app();
    app.fill(3).await;
    let r = app.admin_post(DRAW, &json!({})).await;
    assert_eq!((r.status, r.json()), (StatusCode::CONFLICT, refusal("open")), "open: nobody still to come is left out");
    app.set_open(false).await;
    assert_eq!(app.admin_post(DRAW, &json!({})).await.status, StatusCode::OK, "closed");
    assert_eq!(app.admin_post(DRAW, &json!({})).await.status, StatusCode::OK, "closed again");
    assert_eq!(app.admin_post(DRAW, &json!({ "note": "ignored" })).await.status, StatusCode::OK, "extra fields are ignored");
    for body in ["", "not json", "[]", "null", "42", "\"x\"", "{"] {
        let r = app.admin_raw("POST", DRAW, Some(KEY), body).await;
        assert_eq!((r.status, r.json()), (StatusCode::BAD_REQUEST, refusal("json")), "{body:?}");
    }
    assert_eq!(app.list().await["draws"].as_array().unwrap().len(), 3, "a refused body draws nothing");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_draws_pick_distinct_winners_with_distinct_numbers() {
    let app = app();
    app.fill(12).await;
    app.set_open(false).await;
    let bearer = format!("Bearer {KEY}");
    let tasks: Vec<_> = (0..12)
        .map(|_| {
            let (state, bearer) = (app.state.clone(), bearer.clone());
            tokio::spawn(async move {
                let headers = [("authorization", bearer.as_str()), ("content-type", "application/json")];
                send_to(state, request("POST", DRAW, &headers, b"{}".to_vec())).await
            })
        })
        .collect();
    let mut numbers = BTreeSet::new();
    let mut winners = BTreeSet::new();
    for t in tasks {
        let r = t.await.unwrap();
        assert_eq!(r.status, StatusCode::OK);
        let j = r.json();
        numbers.insert(j["draw"]["n"].as_i64().unwrap());
        winners.insert(j["winner"]["id"].as_i64().unwrap());
    }
    assert_eq!(numbers, (1..=12).collect::<BTreeSet<i64>>(), "draw numbers 1..12, none twice");
    assert_eq!(winners, (1..=12).collect::<BTreeSet<i64>>(), "twelve draws over twelve entries: everyone once");
    assert_eq!(app.admin_post(DRAW, &json!({})).await.status, StatusCode::CONFLICT);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_entries_for_one_email_leave_exactly_one_row() {
    let app = app();
    let tasks: Vec<_> = (1..=20u32)
        .map(|n| {
            let state = app.state.clone();
            tokio::spawn(async move {
                let mut body = valid_entry(n);
                body["email"] = json!(if n % 2 == 0 { "Same@Example.com" } else { "same@example.com " });
                send_to(state, request("POST", "/api/entry", &[("content-type", "text/plain")], body.to_string().into_bytes())).await
            })
        })
        .collect();
    let mut statuses = Vec::new();
    for t in tasks {
        statuses.push(t.await.unwrap().json()["status"].as_str().unwrap().to_string());
    }
    assert_eq!(statuses.iter().filter(|s| *s == "new").count(), 1, "{statuses:?}");
    assert_eq!(statuses.iter().filter(|s| *s == "taken").count(), 19);
    assert_eq!(app.list().await["count"], 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn parallel_resends_of_one_entry_are_one_new_and_the_rest_retry() {
    let app = app();
    let tasks: Vec<_> = (0..20)
        .map(|_| {
            let state = app.state.clone();
            tokio::spawn(async move { send_to(state, request("POST", "/api/entry", &[], valid_entry(7).to_string().into_bytes())).await })
        })
        .collect();
    let mut statuses = Vec::new();
    for t in tasks {
        statuses.push(t.await.unwrap().json()["status"].as_str().unwrap().to_string());
    }
    assert_eq!((statuses.iter().filter(|s| *s == "new").count(), statuses.iter().filter(|s| *s == "retry").count()), (1, 19), "{statuses:?}");
    assert_eq!(app.list().await["count"], 1);
}

#[tokio::test]
async fn clear_refuses_while_entries_are_open_and_deletes_nothing() {
    let app = app();
    app.fill(3).await;
    app.set_open(false).await;
    app.admin_post(DRAW, &json!({})).await;
    app.set_open(true).await;
    let r = app.admin_post(CLEAR, &json!({ "confirm": "DZĒST" })).await;
    assert_eq!((r.status, r.json()), (StatusCode::CONFLICT, refusal("open")));
    let list = app.list().await;
    assert_eq!((list["count"].as_i64(), list["draws"].as_array().unwrap().len()), (Some(3), 1));
}

#[tokio::test]
async fn clear_needs_the_exact_word() {
    let app = app();
    app.fill(2).await;
    app.set_open(false).await;
    let wrong = [json!({}), json!({ "confirm": "" }), json!({ "confirm": null }), json!({ "confirm": "dzēst" }), json!({ "confirm": "DZEST" }), json!({ "confirm": "DZĒST " }), json!({ "confirm": " DZĒST" }), json!({ "confirm": "DZĒSTDZĒST" }), json!({ "confirm": "yes" })];
    for body in wrong {
        let r = app.admin_post(CLEAR, &body).await;
        assert_eq!((r.status, r.json()), (StatusCode::UNPROCESSABLE_ENTITY, refusal("confirm")), "{body}");
    }
    assert_eq!(app.list().await["count"], 2, "nothing was deleted");
    // Validation comes before the state check: a wrong word is a 422 even while entries are open.
    app.set_open(true).await;
    assert_eq!(app.admin_post(CLEAR, &json!({ "confirm": "no" })).await.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn clear_with_the_word_on_closed_entries_deletes_every_entry_and_draw() {
    let app = app();
    app.fill(4).await;
    app.set_open(false).await;
    app.admin_post(DRAW, &json!({})).await;
    app.admin_post(DRAW, &json!({})).await;
    let r = app.admin_post(CLEAR, &json!({ "confirm": "DZĒST" })).await;
    assert_eq!((r.status, r.json()), (StatusCode::OK, json!({ "ok": true, "deleted": 4 })));
    assert_eq!(app.list().await, json!({ "ok": true, "open": false, "count": 0, "eligible": 0, "entries": [], "draws": [] }), "closed stays closed");
    assert_eq!(app.admin_get("/api/admin/export.xlsx").await.status, StatusCode::OK);
    assert_eq!(app.admin_post(CLEAR, &json!({ "confirm": "DZĒST" })).await.json(), json!({ "ok": true, "deleted": 0 }), "clearing nothing is fine");
    // A new round starts clean: entry 1 and draw 1 again.
    app.set_open(true).await;
    app.fill(2).await;
    app.set_open(false).await;
    let j = app.admin_post(DRAW, &json!({})).await.json();
    assert_eq!(j["draw"]["n"], 1);
    assert_eq!(ids(&app.list().await, "won").len(), 1);
    assert_eq!(app.list().await["entries"][0]["id"], 1);
}

#[tokio::test]
async fn bad_json_on_clear_is_400() {
    let app = app();
    app.set_open(false).await;
    for body in ["", "not json", "[]", "null", r#"{"confirm":5}"#, r#"{"confirm":["DZĒST"]}"#] {
        let r = app.admin_raw("POST", CLEAR, Some(KEY), body).await;
        assert_eq!((r.status, r.json()), (StatusCode::BAD_REQUEST, refusal("json")), "{body:?}");
    }
}
