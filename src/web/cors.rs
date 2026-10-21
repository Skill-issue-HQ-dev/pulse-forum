//! CORS for the page on another origin: the allow list, the preflight answer and the headers every answer carries.
//! Owns origin matching and the CORS header set; the list comes from PULSE_ORIGINS.
//! Invariant: Access-Control-Allow-Origin is only ever a listed origin, never `*`; Vary: Origin is on every answer.

use super::Shared;
use crate::config;
use axum::extract::{Request, State};
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// One origin as it is compared: trimmed, no trailing slash, lower case (browsers send it lower case).
pub fn normalize(origin: &str) -> String {
    origin.trim().trim_end_matches('/').to_lowercase()
}

/// A comma separated origin list, normalized, blanks skipped.
pub fn parse_origins(list: &str) -> Vec<String> {
    list.split(',').map(normalize).filter(|o| !o.is_empty()).collect()
}

/// Startup check of the list: every entry must be `scheme://host[:port]`, no path and no `*` (a typo here would
/// silently stop the page from reading any answer on the day).
pub fn check_origins(origins: &[String]) -> Result<(), String> {
    for origin in origins {
        let host = origin.split_once("://").map_or("", |(_, rest)| rest);
        if host.is_empty() || host.contains('/') || host.contains('*') {
            return Err(format!("serve: {} entry {origin:?} is not an origin like https://host (no path, no *)", config::ORIGINS_ENV));
        }
    }
    Ok(())
}

/// Answers every OPTIONS request itself (204) and adds the CORS headers to every answer.
pub async fn apply(State(s): State<Shared>, req: Request, next: Next) -> Response {
    let allowed = listed(&s.origins, req.headers());
    let mut res = if req.method() == Method::OPTIONS { preflight() } else { next.run(req).await };
    let headers = res.headers_mut();
    headers.insert(header::VARY, HeaderValue::from_static("Origin"));
    headers.insert(header::ACCESS_CONTROL_EXPOSE_HEADERS, HeaderValue::from_static(config::CORS_EXPOSE));
    if let Some(origin) = allowed {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin);
    }
    res
}

/// The request's Origin, when it is on the list (the listed spelling is what gets echoed).
fn listed(origins: &[String], headers: &HeaderMap) -> Option<HeaderValue> {
    let sent = normalize(headers.get(header::ORIGIN)?.to_str().ok()?);
    if sent == "*" {
        return None;
    }
    let found = origins.iter().find(|o| **o == sent)?;
    HeaderValue::from_str(found).ok()
}

fn preflight() -> Response {
    let allow = [
        (header::ACCESS_CONTROL_ALLOW_METHODS, config::CORS_METHODS),
        (header::ACCESS_CONTROL_ALLOW_HEADERS, config::CORS_HEADERS),
        (header::ACCESS_CONTROL_MAX_AGE, config::CORS_MAX_AGE),
    ];
    (StatusCode::NO_CONTENT, allow).into_response()
}
