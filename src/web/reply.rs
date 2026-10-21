//! The contract's answer envelope (`{"ok":true,...}` and `{"ok":false,"error":CODE}`) and request-body decoding.
//! Owns the pairing of status code and error code for the shared failures: json, too_large, server, not_found.
//! Invariant: an answer never echoes request content, and a 500 logs only its stage-tagged cause.

use axum::body::Bytes;
use axum::extract::rejection::BytesRejection;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

/// 200 with this JSON body.
pub fn ok(value: Value) -> Response {
    (StatusCode::OK, Json(value)).into_response()
}

/// 200 `{"ok":true,"status":WORD}`: the entry endpoint's answer (new, retry, taken).
pub fn status(word: &str) -> Response {
    ok(json!({ "ok": true, "status": word }))
}

/// A refusal: this status with `{"ok":false,"error":CODE}`.
pub fn refuse(status: StatusCode, code: &str) -> Response {
    (status, Json(json!({ "ok": false, "error": code }))).into_response()
}

/// 500 `{"ok":false,"error":"server"}`; the cause (it names its stage, never a person's data) goes to the log.
pub fn fail(cause: &str) -> Response {
    eprintln!("{cause}");
    refuse(StatusCode::INTERNAL_SERVER_ERROR, "server")
}

/// The request body as JSON, whatever its content type. Too large is 413 `too_large`; unreadable, not JSON
/// or the wrong field types is 400 `json`. The refusal is logged by code only, never by content, and boxed
/// because a Response is large.
pub fn parse<T: DeserializeOwned>(body: Result<Bytes, BytesRejection>) -> Result<T, Box<Response>> {
    let bytes = match body {
        Ok(bytes) => bytes,
        Err(rejection) if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE => {
            return Err(refused_body(StatusCode::PAYLOAD_TOO_LARGE, "too_large"));
        }
        Err(_) => return Err(refused_body(StatusCode::BAD_REQUEST, "json")),
    };
    serde_json::from_slice(&bytes).map_err(|_| refused_body(StatusCode::BAD_REQUEST, "json"))
}

fn refused_body(status: StatusCode, code: &str) -> Box<Response> {
    println!("body: refused ({code})");
    Box::new(refuse(status, code))
}
