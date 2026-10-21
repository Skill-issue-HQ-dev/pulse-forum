//! The page's two calls: whether entries are open, and send an entry. JSON in, JSON out.
//! Owns the HTTP side of both (status codes, the answer order, log lines without personal data); the rules live in core::entry.
//! Invariant: the order is parse (400), honeypot (200 new, nothing stored), closed (423), rules (422), store (200).

use super::{db, reply, Shared};
use crate::core::entry::{self, Check, Entry, EntryInput};
use crate::io::clock;
use crate::io::store::{self, Inserted};
use axum::body::Bytes;
use axum::extract::rejection::BytesRejection;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::Response;
use rusqlite::Connection;
use serde_json::json;

/// `GET /api/status`: `{"ok":true,"open":BOOL,"count":N}`; the count is what the start screen shows.
pub async fn status(State(s): State<Shared>) -> Response {
    let conn = db(&s);
    match store::is_open(&conn).and_then(|open| Ok((open, store::count(&conn)?))) {
        Ok((open, count)) => reply::ok(json!({ "ok": true, "open": open, "count": count })),
        Err(e) => reply::fail(&e),
    }
}

/// `POST /api/entry`: the body is JSON whatever its content type (the page sends text/plain to skip the preflight).
pub async fn entry(State(s): State<Shared>, body: Result<Bytes, BytesRejection>) -> Response {
    let input: EntryInput = match reply::parse(body) {
        Ok(input) => input,
        Err(refusal) => return *refusal,
    };
    let verdict = entry::check(input);
    if matches!(verdict, Check::Bot) {
        println!("entry: dropped (honeypot filled)");
        return reply::status("new");
    }
    let conn = db(&s);
    let open = match store::is_open(&conn) {
        Ok(open) => open,
        Err(e) => return reply::fail(&e),
    };
    match verdict {
        Check::Valid(entry) if !open => closed(&conn, &entry.token),
        _ if !open => {
            println!("entry: refused (closed)");
            reply::refuse(StatusCode::LOCKED, "closed")
        }
        Check::Invalid(code) => {
            println!("entry: refused ({code})");
            reply::refuse(StatusCode::UNPROCESSABLE_ENTITY, code)
        }
        Check::Valid(entry) => store_entry(&conn, &entry),
        Check::Bot => reply::status("new"),
    }
}

/// Closed: a phone resending an entry that is already stored [its answer got lost] still hears it is in the draw.
fn closed(conn: &Connection, token: &str) -> Response {
    match store::has_token(conn, token) {
        Ok(true) => {
            println!("entry: retry after closing (token already stored)");
            reply::status("retry")
        }
        Ok(false) => {
            println!("entry: refused (closed)");
            reply::refuse(StatusCode::LOCKED, "closed")
        }
        Err(e) => reply::fail(&e),
    }
}

fn store_entry(conn: &Connection, entry: &Entry) -> Response {
    match store::insert(conn, entry, &clock::now()) {
        Ok(Inserted::New) => {
            println!("entry: new");
            reply::status("new")
        }
        Ok(Inserted::Retry) => {
            println!("entry: retry (token already stored, row unchanged)");
            reply::status("retry")
        }
        Ok(Inserted::Taken) => {
            println!("entry: taken (e-mail already in, first entry kept)");
            reply::status("taken")
        }
        Ok(Inserted::SamePhone(first)) => {
            // The guest hears "new" like anyone else; the organisers see the link and can put the entry back.
            println!("entry: new, same phone as entry {first}: kept out of the draw");
            reply::status("new")
        }
        Err(e) => reply::fail(&e),
    }
}
