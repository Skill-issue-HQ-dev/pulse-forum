//! Shared harness for the web tests: a temp database behind the real router, and small request/answer helpers.
//! Owns the sample entry and the admin key every test uses. Invariant: nothing real in here, only made-up values.
#![allow(dead_code)]

use axum::body::{to_bytes, Body, Bytes};
use axum::http::{HeaderMap, Request, StatusCode};
use pulse_forum::io::store;
use pulse_forum::web::{self, Shared};
use serde_json::{json, Value};
use tower::ServiceExt;

/// The admin key of every test app: 40 characters, made up.
pub const KEY: &str = "kkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkkk";
/// The one browser origin a default test app allows.
pub const ORIGIN: &str = "https://skill-issue-hq-dev.github.io";

/// The router over a fresh database in a temp dir (removed when the App drops).
pub struct App {
    pub dir: tempfile::TempDir,
    pub state: Shared,
}

pub fn app() -> App {
    app_with_origins(&[ORIGIN])
}

pub fn app_with_origins(origins: &[&str]) -> App {
    let dir = tempfile::tempdir().unwrap();
    let conn = store::open(dir.path()).unwrap();
    let origins: Vec<String> = origins.iter().map(|o| o.to_string()).collect();
    App { state: web::state(conn, KEY, &origins), dir }
}

/// One answer, read to the end.
pub struct Reply {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}

impl Reply {
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }
    pub fn header(&self, name: &str) -> String {
        self.headers.get(name).map(|v| v.to_str().unwrap().to_string()).unwrap_or_default()
    }
    pub fn has_header(&self, name: &str) -> bool {
        self.headers.contains_key(name)
    }
}

/// A request with these headers and body.
pub fn request(method: &str, path: &str, headers: &[(&str, &str)], body: Vec<u8>) -> Request<Body> {
    let mut b = Request::builder().method(method).uri(path);
    for (name, value) in headers {
        b = b.header(*name, *value);
    }
    b.body(Body::from(body)).unwrap()
}

/// One request through the router over this state (also what concurrent tests spawn).
pub async fn send_to(state: Shared, req: Request<Body>) -> Reply {
    let res = web::router(state).oneshot(req).await.unwrap();
    let (parts, body) = res.into_parts();
    Reply { status: parts.status, headers: parts.headers, body: to_bytes(body, usize::MAX).await.unwrap() }
}

impl App {
    pub async fn send(&self, req: Request<Body>) -> Reply {
        send_to(self.state.clone(), req).await
    }

    pub async fn get(&self, path: &str) -> Reply {
        self.send(request("GET", path, &[], vec![])).await
    }

    /// A request with a given Origin header (a browser calling from that page).
    pub async fn get_from(&self, path: &str, origin: &str) -> Reply {
        self.send(request("GET", path, &[("origin", origin)], vec![])).await
    }

    /// `POST /api/entry` the way the page sends it: JSON bytes as text/plain, so no preflight.
    pub async fn entry(&self, body: &Value) -> Reply {
        self.entry_raw(body.to_string().into_bytes()).await
    }

    pub async fn entry_raw(&self, body: Vec<u8>) -> Reply {
        self.send(request("POST", "/api/entry", &[("content-type", "text/plain;charset=UTF-8")], body)).await
    }

    /// A valid entry number `n` (own token and e-mail), posted.
    pub async fn submit(&self, n: u32) -> Reply {
        self.entry(&valid_entry(n)).await
    }

    /// `count` valid entries, numbers 1..=count.
    pub async fn fill(&self, count: u32) {
        for n in 1..=count {
            assert_eq!(self.submit(n).await.json()["status"], "new");
        }
    }

    /// An admin call with a bearer key (or none) and a raw body.
    pub async fn admin_raw(&self, method: &str, path: &str, key: Option<&str>, body: &str) -> Reply {
        let auth = key.map(|k| format!("Bearer {k}"));
        let mut headers = vec![("content-type", "application/json")];
        if let Some(a) = auth.as_deref() {
            headers.push(("authorization", a));
        }
        self.send(request(method, path, &headers, body.as_bytes().to_vec())).await
    }

    pub async fn admin_get(&self, path: &str) -> Reply {
        self.admin_raw("GET", path, Some(KEY), "").await
    }

    pub async fn admin_post(&self, path: &str, body: &Value) -> Reply {
        self.admin_raw("POST", path, Some(KEY), &body.to_string()).await
    }

    /// The entries list the admin page reads.
    pub async fn list(&self) -> Value {
        self.admin_get("/api/admin/entries").await.json()
    }

    pub async fn set_open(&self, open: bool) {
        assert_eq!(self.admin_post("/api/admin/open", &json!({ "open": open })).await.status, StatusCode::OK);
    }
}

/// A valid entry body; `n` makes its token, e-mail and phone unique.
pub fn valid_entry(n: u32) -> Value {
    json!({
        "token": format!("{n:032x}"),
        "lang": "lv",
        "category": "friendship",
        "story": "Satiku jaunu cilvēku pasākumā un mēs sākām sarunu par kopīgu projektu.",
        "first": "Testa",
        "last": "Ķirsis",
        "email": format!("test{n}@example.com"),
        "phone": format!("+371 2{n:07}"),
        "company": "Parauga SIA",
        "consent": true,
        "share": false,
        "website": ""
    })
}
