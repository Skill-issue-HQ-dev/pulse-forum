//! The organiser's calls, all behind the admin key: the list, the Excel export, open/close, exclude, delete, draw,
//! clear. Owns the HTTP side of the seven routes and the bearer gate; the data work lives in io::store and core::workbook.
//! Invariant: no route here answers without a valid `Authorization: Bearer <key>`, and no answer may be cached.

use super::{db, reply, Shared};
use crate::config;
use crate::core::auth;
use crate::core::workbook::{self, Facts};
use crate::io::store::{self, Cleared, Deleted, Drawn};
use crate::io::{clock, random};
use axum::body::Bytes;
use axum::extract::rejection::BytesRejection;
use axum::extract::{Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;
use serde_json::{json, Map, Value};

/// The admin routes (mounted under /api/admin), every one behind the key check.
pub fn routes(state: Shared) -> Router<Shared> {
    Router::new()
        .route("/entries", get(entries))
        .route("/export.xlsx", get(export))
        .route("/open", post(open))
        .route("/exclude", post(exclude))
        .route("/delete", post(delete))
        .route("/draw", post(draw))
        .route("/clear", post(clear))
        .layer(middleware::from_fn_with_state(state, require_key))
}

/// 401 `key` unless the request carries the admin key as a bearer token; admin answers are never cached.
async fn require_key(State(s): State<Shared>, req: Request, next: Next) -> Response {
    let presented = req.headers().get(header::AUTHORIZATION);
    let allowed = presented.is_some_and(|v| auth::bearer_ok(v.as_bytes(), &s.admin_key));
    let mut res = if allowed {
        next.run(req).await
    } else {
        println!("admin: refused (no valid key)");
        reply::refuse(StatusCode::UNAUTHORIZED, "key")
    };
    res.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    res
}

/// `GET /entries`: every entry in submission order, every draw, and the counts.
async fn entries(State(s): State<Shared>) -> Response {
    let snap = match store::snapshot(&db(&s)) {
        Ok(snap) => snap,
        Err(e) => return reply::fail(&e),
    };
    println!("admin: entries {} ({} eligible, {} draws)", snap.rows.len(), snap.eligible, snap.draws.len());
    reply::ok(json!({
        "ok": true,
        "open": snap.open,
        "count": snap.rows.len(),
        "eligible": snap.eligible,
        "entries": snap.rows,
        "draws": snap.draws,
    }))
}

/// `GET /export.xlsx`: the three-sheet workbook, named after the Riga wall clock.
async fn export(State(s): State<Shared>) -> Response {
    let snap = match store::snapshot(&db(&s)) {
        Ok(snap) => snap,
        Err(e) => return reply::fail(&e),
    };
    let now = clock::now();
    let facts = Facts { exported_at: now.clone(), open: snap.open };
    match workbook::build(&snap.rows, &snap.draws, &facts) {
        Ok(bytes) => {
            println!("admin: export {} entries, {} draws", snap.rows.len(), snap.draws.len());
            let disposition = format!("attachment; filename=\"{}\"", workbook::file_name(&now));
            ([(header::CONTENT_TYPE, config::XLSX_TYPE.to_string()), (header::CONTENT_DISPOSITION, disposition)], bytes).into_response()
        }
        Err(e) => reply::fail(&e),
    }
}

#[derive(Deserialize)]
struct OpenInput {
    open: bool,
}

/// `POST /open {"open":BOOL}`: opens or closes entries.
async fn open(State(s): State<Shared>, body: Result<Bytes, BytesRejection>) -> Response {
    let input: OpenInput = match reply::parse(body) {
        Ok(input) => input,
        Err(refusal) => return *refusal,
    };
    match store::set_open(&db(&s), input.open) {
        Ok(()) => {
            println!("admin: entries {}", if input.open { "opened" } else { "closed" });
            reply::ok(json!({ "ok": true, "open": input.open }))
        }
        Err(e) => reply::fail(&e),
    }
}

#[derive(Deserialize)]
struct ExcludeInput {
    id: i64,
    excluded: bool,
}

/// `POST /exclude {"id":N,"excluded":BOOL}`: takes an entry out of the draw, or puts it back.
async fn exclude(State(s): State<Shared>, body: Result<Bytes, BytesRejection>) -> Response {
    let input: ExcludeInput = match reply::parse(body) {
        Ok(input) => input,
        Err(refusal) => return *refusal,
    };
    match store::exclude(&db(&s), input.id, input.excluded) {
        Ok(true) => {
            println!("admin: entry {} {}", input.id, if input.excluded { "excluded" } else { "back in the draw" });
            reply::ok(json!({ "ok": true }))
        }
        Ok(false) => reply::refuse(StatusCode::NOT_FOUND, "id"),
        Err(e) => reply::fail(&e),
    }
}

#[derive(Deserialize)]
struct DeleteInput {
    id: i64,
}

/// `POST /delete {"id":N}`: deletes one entry at its owner's request; a drawn winner stays.
async fn delete(State(s): State<Shared>, body: Result<Bytes, BytesRejection>) -> Response {
    let input: DeleteInput = match reply::parse(body) {
        Ok(input) => input,
        Err(refusal) => return *refusal,
    };
    match store::delete(&mut db(&s), input.id) {
        Ok(Deleted::Gone) => {
            println!("admin: entry {} deleted", input.id);
            reply::ok(json!({ "ok": true, "deleted": input.id }))
        }
        Ok(Deleted::Missing) => reply::refuse(StatusCode::NOT_FOUND, "id"),
        Ok(Deleted::Winner) => {
            println!("admin: delete refused (entry {} won a draw)", input.id);
            reply::refuse(StatusCode::CONFLICT, "winner")
        }
        Err(e) => reply::fail(&e),
    }
}

/// `POST /draw {}`: one winner, uniformly among the eligible entries, from operating-system randomness; only
/// once entries are closed, so nobody still to come is left out.
async fn draw(State(s): State<Shared>, body: Result<Bytes, BytesRejection>) -> Response {
    if let Err(refusal) = reply::parse::<Map<String, Value>>(body) {
        return *refusal;
    }
    match store::draw(&mut db(&s), &clock::now(), &mut random::next_u64) {
        Ok(Drawn::Picked { draw, winner }) => {
            println!("admin: draw {} picked entry {} of {} eligible", draw.n, winner.id, draw.pool);
            reply::ok(json!({ "ok": true, "draw": draw, "winner": winner }))
        }
        Ok(Drawn::Empty) => {
            println!("admin: draw refused (nobody eligible)");
            reply::refuse(StatusCode::CONFLICT, "empty")
        }
        Ok(Drawn::StillOpen) => {
            println!("admin: draw refused (entries are open)");
            reply::refuse(StatusCode::CONFLICT, "open")
        }
        Err(e) => reply::fail(&e),
    }
}

#[derive(Deserialize)]
struct ClearInput {
    confirm: Option<String>,
}

/// `POST /clear {"confirm":"DZĒST"}`: deletes every entry and draw, but only with the word and only while closed.
async fn clear(State(s): State<Shared>, body: Result<Bytes, BytesRejection>) -> Response {
    let input: ClearInput = match reply::parse(body) {
        Ok(input) => input,
        Err(refusal) => return *refusal,
    };
    if input.confirm.as_deref() != Some(config::CLEAR_WORD) {
        return reply::refuse(StatusCode::UNPROCESSABLE_ENTITY, "confirm");
    }
    match store::clear(&mut db(&s)) {
        Ok(Cleared::Deleted(n)) => {
            println!("admin: cleared {n} entries and every draw");
            reply::ok(json!({ "ok": true, "deleted": n }))
        }
        Ok(Cleared::StillOpen) => {
            println!("admin: clear refused (entries are open)");
            reply::refuse(StatusCode::CONFLICT, "open")
        }
        Err(e) => reply::fail(&e),
    }
}
