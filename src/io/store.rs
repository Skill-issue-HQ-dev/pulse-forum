//! The entries database: one SQLite file holding entries, draws and the open flag.
//! Owns the schema and every read and write: insert, list, exclude, delete, draw, clear, open/closed.
//! Invariant: token and e-mail are UNIQUE (the first entry per e-mail wins, a later one never changes it); an entry
//! whose phone matches an earlier one starts excluded; a draw runs only while closed, and its pick and its row are
//! one transaction that skips excluded entries and earlier winners.

use crate::config;
use crate::core::draw::try_pick;
use crate::core::entry::Entry;
use crate::core::record::{Draw, Row};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::path::Path;

/// secure_delete zeroes freed pages, so `clear` really erases the personal data from the file.
const SCHEMA: &str = "PRAGMA journal_mode = WAL;
PRAGMA busy_timeout = 5000;
PRAGMA foreign_keys = ON;
PRAGMA secure_delete = ON;
CREATE TABLE IF NOT EXISTS entries (
    id INTEGER PRIMARY KEY,
    token TEXT NOT NULL UNIQUE,
    email_key TEXT NOT NULL UNIQUE,
    at TEXT NOT NULL,
    lang TEXT NOT NULL,
    category TEXT NOT NULL,
    story TEXT NOT NULL,
    first TEXT NOT NULL,
    last TEXT NOT NULL,
    email TEXT NOT NULL,
    phone TEXT NOT NULL,
    company TEXT NOT NULL,
    share INTEGER NOT NULL,
    excluded INTEGER NOT NULL DEFAULT 0,
    phone_key TEXT NOT NULL DEFAULT '',
    dup_of INTEGER
);
CREATE TABLE IF NOT EXISTS draws (
    n INTEGER PRIMARY KEY,
    at TEXT NOT NULL,
    pool INTEGER NOT NULL,
    winner_id INTEGER NOT NULL UNIQUE REFERENCES entries(id),
    pool_ids TEXT NOT NULL DEFAULT ''
);
CREATE TABLE IF NOT EXISTS settings (
    name TEXT PRIMARY KEY,
    value TEXT NOT NULL
);";

/// The one definition of "eligible": not excluded and never drawn.
const ELIGIBLE: &str = "excluded = 0 AND id NOT IN (SELECT winner_id FROM draws)";
/// An entry with the number of the draw that picked it (NULL when none did).
const ROW_SELECT: &str = "SELECT e.id, e.at, e.lang, e.category, e.story, e.first, e.last, e.email, e.phone,
    e.company, e.share, e.excluded, d.n, e.dup_of FROM entries e LEFT JOIN draws d ON d.winner_id = e.id";

/// Opens (and creates on first use) `<dir>/entries.sqlite3`. A new file is private to its owner (mode 0600),
/// and entries start open: a database with no `open` setting counts as open.
pub fn open(dir: &Path) -> Result<Connection, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("store: create {}: {e}", dir.display()))?;
    let path = dir.join(config::DB_FILE);
    create_private(&path)?;
    let conn = Connection::open(&path).map_err(|e| format!("store: open {}: {e}", path.display()))?;
    conn.execute_batch(SCHEMA).map_err(|e| format!("store: schema: {e}"))?;
    Ok(conn)
}

fn create_private(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::OpenOptionsExt;
    match std::fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path) {
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(e) => Err(format!("store: create {}: {e}", path.display())),
    }
}

/// Whether entries are being accepted.
pub fn is_open(conn: &Connection) -> Result<bool, String> {
    let value: Option<String> = conn
        .query_row("SELECT value FROM settings WHERE name = 'open'", [], |r| r.get(0))
        .optional()
        .map_err(|e| format!("store: open flag: {e}"))?;
    Ok(value.as_deref() != Some("0"))
}

pub fn set_open(conn: &Connection, open: bool) -> Result<(), String> {
    conn.execute(
        "INSERT INTO settings (name, value) VALUES ('open', ?1) ON CONFLICT (name) DO UPDATE SET value = excluded.value",
        [if open { "1" } else { "0" }],
    )
    .map_err(|e| format!("store: set open: {e}"))?;
    Ok(())
}

/// What an insert did.
#[derive(Debug, PartialEq)]
pub enum Inserted {
    /// A new row.
    New,
    /// This token is already stored (the phone resent after a lost answer): nothing changed.
    Retry,
    /// This e-mail is already in under another token: the first entry stays as it was.
    Taken,
    /// A new row, but this entry (its id) has the same phone: the new row starts out of the draw.
    SamePhone(i64),
}

/// Stores a checked entry; a known token or e-mail leaves the stored row untouched. One statement, so the
/// same-phone check and the row cannot be split by another insert.
pub fn insert(conn: &Connection, e: &Entry, now: &str) -> Result<Inserted, String> {
    let changed = conn
        .execute(
            "INSERT INTO entries (token, email_key, at, lang, category, story, first, last, email, phone, company, share,
                 phone_key, dup_of, excluded)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13,
                 (SELECT MIN(id) FROM entries WHERE phone_key = ?13 AND ?13 <> ''),
                 EXISTS (SELECT 1 FROM entries WHERE phone_key = ?13 AND ?13 <> ''))
             ON CONFLICT DO NOTHING",
            params![e.token, e.email_key, now, e.lang, e.category, e.story, e.first, e.last, e.email, e.phone, e.company, e.share, e.phone_key],
        )
        .map_err(|err| format!("store: insert: {err}"))?;
    if changed == 1 {
        let dup_of: Option<i64> = conn
            .query_row("SELECT dup_of FROM entries WHERE token = ?1", [&e.token], |r| r.get(0))
            .map_err(|err| format!("store: insert: same-phone lookup: {err}"))?;
        return Ok(dup_of.map_or(Inserted::New, Inserted::SamePhone));
    }
    // A unique key refused it: the same token is the phone's own resend, anything else another phone's e-mail.
    Ok(if has_token(conn, &e.token)? { Inserted::Retry } else { Inserted::Taken })
}

/// Whether an entry with this token is stored [a resend after a lost answer].
pub fn has_token(conn: &Connection, token: &str) -> Result<bool, String> {
    conn.query_row("SELECT EXISTS (SELECT 1 FROM entries WHERE token = ?1)", [token], |r| r.get(0))
        .map_err(|err| format!("store: token lookup: {err}"))
}

fn to_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Row> {
    Ok(Row {
        id: r.get(0)?,
        at: r.get(1)?,
        lang: r.get(2)?,
        category: r.get(3)?,
        story: r.get(4)?,
        first: r.get(5)?,
        last: r.get(6)?,
        email: r.get(7)?,
        phone: r.get(8)?,
        company: r.get(9)?,
        share: r.get(10)?,
        excluded: r.get(11)?,
        won: r.get(12)?,
        dup_of: r.get(13)?,
    })
}

/// Every entry in submission order.
pub fn list(conn: &Connection) -> Result<Vec<Row>, String> {
    let mut stmt = conn.prepare(&format!("{ROW_SELECT} ORDER BY e.id")).map_err(|e| format!("store: list: {e}"))?;
    let rows = stmt.query_map([], to_row).map_err(|e| format!("store: list: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| format!("store: list: {e}"))
}

pub fn get(conn: &Connection, id: i64) -> Result<Option<Row>, String> {
    conn.query_row(&format!("{ROW_SELECT} WHERE e.id = ?1"), [id], to_row)
        .optional()
        .map_err(|e| format!("store: get {id}: {e}"))
}

/// Every draw in order.
pub fn draws(conn: &Connection) -> Result<Vec<Draw>, String> {
    let mut stmt = conn.prepare("SELECT n, at, pool, winner_id, pool_ids FROM draws ORDER BY n").map_err(|e| format!("store: draws: {e}"))?;
    let rows = stmt
        .query_map([], |r| {
            let ids: String = r.get(4)?;
            Ok(Draw { n: r.get(0)?, at: r.get(1)?, pool: r.get(2)?, winner_id: r.get(3)?, pool_ids: parse_ids(&ids) })
        })
        .map_err(|e| format!("store: draws: {e}"))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| format!("store: draws: {e}"))
}

fn parse_ids(text: &str) -> Vec<i64> {
    text.split(',').filter_map(|id| id.parse().ok()).collect()
}

fn join_ids(ids: &[i64]) -> String {
    ids.iter().map(i64::to_string).collect::<Vec<_>>().join(",")
}

/// Everything the admin list and the export show, read together.
pub struct Snapshot {
    pub open: bool,
    pub rows: Vec<Row>,
    pub draws: Vec<Draw>,
    pub eligible: usize,
}

pub fn snapshot(conn: &Connection) -> Result<Snapshot, String> {
    let eligible: i64 = conn
        .query_row(&format!("SELECT COUNT(*) FROM entries WHERE {ELIGIBLE}"), [], |r| r.get(0))
        .map_err(|e| format!("store: eligible count: {e}"))?;
    Ok(Snapshot { open: is_open(conn)?, rows: list(conn)?, draws: draws(conn)?, eligible: eligible as usize })
}

/// How many entries are stored: the one number the public status shows (no names, nothing else).
pub fn count(conn: &Connection) -> Result<i64, String> {
    conn.query_row("SELECT COUNT(*) FROM entries", [], |r| r.get(0)).map_err(|e| format!("store: count: {e}"))
}

/// Sets or lifts an entry's exclusion from the draw; false when there is no such entry.
pub fn exclude(conn: &Connection, id: i64, excluded: bool) -> Result<bool, String> {
    let changed = conn
        .execute("UPDATE entries SET excluded = ?1 WHERE id = ?2", params![excluded, id])
        .map_err(|e| format!("store: exclude {id}: {e}"))?;
    Ok(changed == 1)
}

/// The result of deleting one entry.
#[derive(Debug, PartialEq)]
pub enum Deleted {
    Gone,
    /// No entry has this id.
    Missing,
    /// A draw picked this entry: it stays, so the draw keeps its winner.
    Winner,
}

/// Deletes one entry at its owner's request [the privacy text promises it]; a drawn winner is refused.
pub fn delete(conn: &mut Connection, id: i64) -> Result<Deleted, String> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| format!("store: delete {id}: begin: {e}"))?;
    let won: Option<bool> = tx
        .query_row("SELECT EXISTS (SELECT 1 FROM draws WHERE winner_id = ?1) FROM entries WHERE id = ?1", [id], |r| r.get(0))
        .optional()
        .map_err(|e| format!("store: delete {id}: {e}"))?;
    match won {
        None => return Ok(Deleted::Missing),
        Some(true) => return Ok(Deleted::Winner),
        Some(false) => {}
    }
    tx.execute("DELETE FROM entries WHERE id = ?1", [id]).map_err(|e| format!("store: delete {id}: {e}"))?;
    tx.commit().map_err(|e| format!("store: delete {id}: commit: {e}"))?;
    checkpoint(conn, "delete");
    Ok(Deleted::Gone)
}

/// The result of a draw (one short-lived value per call, so the size gap between the variants costs nothing).
#[derive(Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Drawn {
    /// Entries are still open: a draw before the close would leave out everyone still to come.
    StillOpen,
    /// Nobody is eligible: nothing was written.
    Empty,
    Picked { draw: Draw, winner: Row },
}

/// Draws one winner uniformly among the eligible entries. The pick and the draw row are one transaction, so
/// a failed random read or a failed write leaves no trace. `next` is the source of random words.
pub fn draw(conn: &mut Connection, now: &str, next: &mut impl FnMut() -> Result<u64, String>) -> Result<Drawn, String> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| format!("store: draw: begin: {e}"))?;
    if is_open(&tx)? {
        return Ok(Drawn::StillOpen);
    }
    let pool = eligible_ids(&tx)?;
    let picked = try_pick(pool.len(), next).map_err(|e| format!("store: draw: {e}"))?;
    let Some(index) = picked else { return Ok(Drawn::Empty) };
    let winner_id = pool[index];
    let n: i64 = tx
        .query_row("SELECT COALESCE(MAX(n), 0) + 1 FROM draws", [], |r| r.get(0))
        .map_err(|e| format!("store: draw: number: {e}"))?;
    let record = Draw { n, at: now.to_string(), pool: pool.len() as i64, winner_id, pool_ids: pool.clone() };
    let row = params![record.n, record.at, record.pool, record.winner_id, join_ids(&pool)];
    tx.execute("INSERT INTO draws (n, at, pool, winner_id, pool_ids) VALUES (?1, ?2, ?3, ?4, ?5)", row)
        .map_err(|e| format!("store: draw: insert: {e}"))?;
    let winner = get(&tx, winner_id)?.ok_or_else(|| format!("store: draw: winner {winner_id} vanished"))?;
    tx.commit().map_err(|e| format!("store: draw: commit: {e}"))?;
    Ok(Drawn::Picked { draw: record, winner })
}

fn eligible_ids(conn: &Connection) -> Result<Vec<i64>, String> {
    let mut stmt = conn
        .prepare(&format!("SELECT id FROM entries WHERE {ELIGIBLE} ORDER BY id"))
        .map_err(|e| format!("store: draw: pool: {e}"))?;
    let ids = stmt.query_map([], |r| r.get(0)).map_err(|e| format!("store: draw: pool: {e}"))?;
    ids.collect::<Result<Vec<_>, _>>().map_err(|e| format!("store: draw: pool: {e}"))
}

/// The result of a clear.
#[derive(Debug, PartialEq)]
pub enum Cleared {
    /// Entries are still open: nothing was deleted.
    StillOpen,
    /// Every entry and every draw is gone; this many entries were deleted.
    Deleted(usize),
}

/// Deletes every entry and draw, but only while entries are closed. The next entry is number 1 again.
pub fn clear(conn: &mut Connection) -> Result<Cleared, String> {
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate).map_err(|e| format!("store: clear: begin: {e}"))?;
    if is_open(&tx)? {
        return Ok(Cleared::StillOpen);
    }
    tx.execute("DELETE FROM draws", []).map_err(|e| format!("store: clear: draws: {e}"))?;
    let deleted = tx.execute("DELETE FROM entries", []).map_err(|e| format!("store: clear: entries: {e}"))?;
    tx.commit().map_err(|e| format!("store: clear: commit: {e}"))?;
    checkpoint(conn, "clear");
    Ok(Cleared::Deleted(deleted))
}

/// Folds the write-ahead log back in so deleted rows do not linger there; failing only leaves them a while longer.
fn checkpoint(conn: &Connection, what: &str) {
    if let Err(e) = conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(())) {
        eprintln!("store: {what}: checkpoint: {e}");
    }
}
