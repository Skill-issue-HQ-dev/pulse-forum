//! Tests for web::cors: the allow list is echoed (never `*`), Vary: Origin rides on every answer, and OPTIONS is
//! a 204 preflight on any path without the admin key; the startup check of the origin list.

mod common;

use axum::http::StatusCode;
use common::{app, app_with_origins, request, valid_entry, KEY, ORIGIN};
use pulse_forum::web::{check_origins, parse_origins};

const ALLOW: &str = "access-control-allow-origin";

#[tokio::test]
async fn a_listed_origin_is_echoed_on_every_kind_of_answer() {
    let app = app();
    let from = [("origin", ORIGIN)];
    let mut bad = valid_entry(9);
    bad["lang"] = serde_json::json!("de");
    let answers = [
        ("GET /healthz", app.send(request("GET", "/healthz", &from, vec![])).await),
        ("POST ok", app.send(request("POST", "/api/entry", &[("origin", ORIGIN), ("content-type", "text/plain")], valid_entry(1).to_string().into_bytes())).await),
        ("POST 422", app.send(request("POST", "/api/entry", &from, bad.to_string().into_bytes())).await),
        ("POST 400", app.send(request("POST", "/api/entry", &from, b"nope".to_vec())).await),
        ("POST 413", app.send(request("POST", "/api/entry", &from, vec![b' '; 20_000])).await),
        ("GET 404", app.send(request("GET", "/nope", &from, vec![])).await),
        ("GET 405", app.send(request("GET", "/api/entry", &from, vec![])).await),
        ("GET 401", app.send(request("GET", "/api/admin/entries", &from, vec![])).await),
    ];
    for (what, r) in &answers {
        assert_eq!(r.header(ALLOW), ORIGIN, "{what}");
        assert_eq!(r.header("vary"), "Origin", "{what}");
        assert_eq!(r.header("access-control-expose-headers"), "content-disposition", "{what}");
    }
}

#[tokio::test]
async fn another_origin_gets_no_allow_header_but_vary() {
    let app = app();
    for origin in [
        "https://evil.example",
        "http://skill-issue-hq-dev.github.io",
        "https://skill-issue-hq-dev.github.io.evil.example",
        "https://sub.skill-issue-hq-dev.github.io",
        "https://skill-issue-hq-dev.github.io:8443",
        "null",
        "*",
        "",
    ] {
        let r = app.get_from("/healthz", origin).await;
        assert_eq!(r.status, StatusCode::OK, "{origin:?}");
        assert!(!r.has_header(ALLOW), "{origin:?} must not be echoed");
        assert_eq!(r.header("vary"), "Origin", "{origin:?}");
    }
    let r = app.get("/healthz").await;
    assert!(!r.has_header(ALLOW), "no Origin header, no allow header");
    assert_eq!(r.header("vary"), "Origin");
}

#[tokio::test]
async fn cors_is_a_browser_rule_so_the_entry_is_still_stored() {
    let app = app();
    let r = app.send(request("POST", "/api/entry", &[("origin", "https://evil.example")], valid_entry(1).to_string().into_bytes())).await;
    assert_eq!(r.json()["status"], "new");
    assert!(!r.has_header(ALLOW), "only the browser is told it may not read the answer");
    assert_eq!(app.list().await["count"], 1);
}

#[tokio::test]
async fn the_allow_header_is_never_a_wildcard() {
    let app = app_with_origins(&[ORIGIN]);
    for origin in [ORIGIN, "*", "https://evil.example"] {
        assert_ne!(app.get_from("/healthz", origin).await.header(ALLOW), "*", "{origin}");
    }
    // Even a list that (wrongly) names `*` cannot make the service echo it.
    let wild = app_with_origins(&["*", ORIGIN]);
    assert!(!wild.get_from("/healthz", "*").await.has_header(ALLOW));
    assert_eq!(wild.get_from("/healthz", ORIGIN).await.header(ALLOW), ORIGIN);
}

#[tokio::test]
async fn options_is_204_with_the_allow_headers_on_any_path() {
    let app = app();
    let paths = ["/", "/healthz", "/api/entry", "/api/status", "/api/admin/entries", "/api/admin/draw", "/api/admin/export.xlsx", "/api/admin/nope", "/nope", "/a/b/c"];
    for path in paths {
        for origin in [Some(ORIGIN), Some("https://evil.example"), None] {
            let mut headers = vec![("access-control-request-method", "POST"), ("access-control-request-headers", "authorization, content-type")];
            if let Some(o) = origin {
                headers.push(("origin", o));
            }
            let r = app.send(request("OPTIONS", path, &headers, vec![])).await;
            assert_eq!(r.status, StatusCode::NO_CONTENT, "{path} {origin:?}");
            assert!(r.body.is_empty(), "{path}");
            assert_eq!(r.header("access-control-allow-methods"), "GET, POST, OPTIONS", "{path}");
            assert_eq!(r.header("access-control-allow-headers"), "authorization, content-type", "{path}");
            assert_eq!(r.header("access-control-max-age"), "600", "{path}");
            assert_eq!(r.header("vary"), "Origin", "{path}");
            assert_eq!(r.header(ALLOW), if origin == Some(ORIGIN) { ORIGIN } else { "" }, "{path} {origin:?}: allow-origin only for a listed origin");
            assert_eq!(r.header("x-content-type-options"), "nosniff", "{path}");
        }
    }
}

#[tokio::test]
async fn a_preflight_needs_no_key_and_changes_nothing() {
    let app = app();
    app.fill(2).await;
    for path in ["/api/admin/draw", "/api/admin/clear", "/api/admin/open", "/api/admin/exclude"] {
        let r = app.send(request("OPTIONS", path, &[("origin", ORIGIN)], vec![])).await;
        assert_eq!(r.status, StatusCode::NO_CONTENT, "{path}");
    }
    let list = app.list().await;
    assert_eq!((list["count"].as_i64(), list["open"].as_bool(), list["draws"].as_array().map(Vec::len)), (Some(2), Some(true), Some(0)));
}

#[tokio::test]
async fn several_origins_each_echo_only_themselves() {
    let app = app_with_origins(&["https://a.example", "http://localhost:5173"]);
    for origin in ["https://a.example", "http://localhost:5173"] {
        assert_eq!(app.get_from("/healthz", origin).await.header(ALLOW), origin);
    }
    assert!(!app.get_from("/healthz", "http://localhost:5174").await.has_header(ALLOW));
}

#[tokio::test]
async fn the_list_is_matched_without_regard_to_case_or_a_trailing_slash() {
    let app = app_with_origins(&["https://Pages.Example/"]);
    assert_eq!(app.get_from("/healthz", "https://pages.example").await.header(ALLOW), "https://pages.example");
    assert_eq!(app.get_from("/healthz", "HTTPS://PAGES.EXAMPLE").await.header(ALLOW), "https://pages.example");
    assert_eq!(parse_origins(" https://A.example/ ,https://b.example,, \n"), ["https://a.example", "https://b.example"]);
    assert!(parse_origins("").is_empty() && parse_origins(" , ,").is_empty());
}

#[tokio::test]
async fn an_origin_header_with_odd_bytes_is_ignored_not_fatal() {
    let app = app();
    let r = app.send(request("GET", "/healthz", &[("origin", "https://évil.example")], vec![])).await;
    assert_eq!(r.status, StatusCode::OK);
    assert!(!r.has_header(ALLOW));
}

#[tokio::test]
async fn the_admin_page_can_read_a_401_and_the_download_name() {
    let app = app();
    let r = app.send(request("GET", "/api/admin/entries", &[("origin", ORIGIN)], vec![])).await;
    assert_eq!((r.status, r.header(ALLOW).as_str()), (StatusCode::UNAUTHORIZED, ORIGIN));
    let auth = format!("Bearer {KEY}");
    let r = app.send(request("GET", "/api/admin/export.xlsx", &[("origin", ORIGIN), ("authorization", auth.as_str())], vec![])).await;
    assert_eq!(r.status, StatusCode::OK);
    assert_eq!(r.header(ALLOW), ORIGIN);
    assert_eq!(r.header("access-control-expose-headers"), "content-disposition");
    assert!(r.header("content-disposition").starts_with("attachment; filename=\"PULSE izloze "));
}

#[test]
fn the_startup_check_accepts_real_origins_and_rejects_typos() {
    let ok = |list: &str| check_origins(&parse_origins(list));
    assert!(ok("https://skill-issue-hq-dev.github.io").is_ok());
    assert!(ok("https://a.example, http://localhost:5173").is_ok());
    for bad in ["*", "https://*", "skill-issue-hq-dev.github.io", "https://a.example/pulse-forum", "https://", "ftp"] {
        let err = ok(bad).unwrap_err();
        assert!(err.contains("PULSE_ORIGINS"), "{bad}: {err}");
    }
}
