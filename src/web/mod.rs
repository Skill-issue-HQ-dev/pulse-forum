//! Web edge: the router, shared state, the security headers, the body limit and the loopback listener.
//! Owns the route table and the layer order (security headers > CORS > body limit > admin key > handler).
//! Invariant: every answer (404, 405, 413 and OPTIONS included) carries the security headers and Vary: Origin.

mod admin;
mod api;
mod cors;
mod reply;

pub use cors::{check_origins, parse_origins};

use crate::config;
use crate::io::store;
use axum::extract::{DefaultBodyLimit, Request};
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use rusqlite::Connection;
use serde_json::json;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

/// The one database connection every request shares, the admin key and the allowed browser origins.
pub struct AppState {
    pub db: Mutex<Connection>,
    pub admin_key: String,
    pub origins: Vec<String>,
}
pub type Shared = Arc<AppState>;

/// Shared state for this database; the origins are normalized the way requests' Origin headers are.
pub fn state(conn: Connection, admin_key: &str, origins: &[String]) -> Shared {
    let origins = origins.iter().map(|o| cors::normalize(o)).collect();
    Arc::new(AppState { db: Mutex::new(conn), admin_key: admin_key.to_string(), origins })
}

/// The connection, locked. A request that panicked cannot leave it half-used (its transaction rolls back
/// on drop), so a poisoned lock is taken over instead of failing every later request.
pub(crate) fn db(s: &AppState) -> MutexGuard<'_, Connection> {
    s.db.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// All routes: health, version, the page's two calls, the admin calls; headers, CORS and the body limit around them.
pub fn router(state: Shared) -> Router {
    Router::new()
        .route("/healthz", get(|| async { reply::ok(json!({ "ok": true })) }))
        .route("/version", get(|| async { Json(json!({ "version": env!("CARGO_PKG_VERSION") })) }))
        .route("/api/status", get(api::status))
        .route("/api/entry", post(api::entry))
        .nest("/api/admin", admin::routes(state.clone()))
        .fallback(|| async { reply::refuse(StatusCode::NOT_FOUND, "not_found") })
        .layer(DefaultBodyLimit::max(config::MAX_BODY_BYTES))
        .with_state(state.clone())
        .layer(middleware::from_fn_with_state(state, cors::apply))
        .layer(middleware::from_fn(security_headers))
}

async fn security_headers(req: Request, next: Next) -> Response {
    let mut res = next.run(req).await;
    for (name, value) in config::SECURITY_HEADERS {
        res.headers_mut().insert(HeaderName::from_static(name), HeaderValue::from_static(value));
    }
    res
}

/// What `serve` needs, as `main` reads it from the environment.
pub struct Settings {
    pub port: u16,
    pub data: PathBuf,
    pub admin_key: String,
    pub origins: Vec<String>,
}

/// The admin key must hold at least config::MIN_KEY_CHARS characters; the message never shows the key.
pub fn check_admin_key(key: &str) -> Result<(), String> {
    let chars = key.chars().count();
    if chars < config::MIN_KEY_CHARS {
        return Err(format!("serve: {} must hold at least {} characters (it has {chars})", config::ADMIN_KEY_ENV, config::MIN_KEY_CHARS));
    }
    Ok(())
}

/// Checks the key and the origin list, opens the database and serves on 127.0.0.1 until the process stops.
pub async fn serve(settings: &Settings) -> Result<(), String> {
    check_admin_key(&settings.admin_key)?;
    check_origins(&settings.origins)?;
    let shared = state(store::open(&settings.data)?, &settings.admin_key, &settings.origins);
    let addr = SocketAddr::from(([127, 0, 0, 1], settings.port));
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| format!("serve: bind {addr}: {e}"))?;
    let bound = listener.local_addr().map_err(|e| format!("serve: local address: {e}"))?;
    println!("serve: http://{bound}, data {}, {} allowed origin(s)", settings.data.display(), shared.origins.len());
    axum::serve(listener, router(shared)).await.map_err(|e| format!("serve: {e}"))
}
