//! Tests for io::store: first e-mail wins (new/retry/taken), one chance per phone (checked through an index), the
//! open flag, exclude, delete, the draw (only while closed, skips excluded entries and earlier winners, numbered
//! from 1, keeps its pool, one transaction), clear, and a private database file.

use pulse_forum::core::entry::{check, Check, Entry, EntryInput};
use pulse_forum::io::store::{self, Cleared, Deleted, Drawn, Inserted};
use rusqlite::Connection;
use serde_json::json;
use std::os::unix::fs::PermissionsExt;

const T0: &str = "2026-10-28T10:00:00Z";

fn entry_with(n: u32, token_of: u32, email: &str, story: &str) -> Entry {
    entry_full(n, token_of, email, story, &format!("+371 2{n:07}"))
}

fn entry_full(n: u32, token_of: u32, email: &str, story: &str, phone: &str) -> Entry {
    let v = json!({
        "token": format!("{token_of:032x}"), "lang": "lv", "category": "trip", "story": story,
        "first": format!("Testa{n}"), "last": "Ķirsis", "email": email, "phone": phone,
        "company": "Parauga SIA", "consent": true, "share": n & 1 == 0, "website": ""
    });
    match check(serde_json::from_value::<EntryInput>(v).unwrap()) {
        Check::Valid(e) => e,
        other => panic!("{other:?}"),
    }
}

fn entry(n: u32) -> Entry {
    entry_with(n, n, &format!("test{n}@example.com"), "Satiku jaunu cilvēku pasākumā un mēs runājām.")
}

fn db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = store::open(dir.path()).unwrap();
    (dir, conn)
}

fn fill(conn: &Connection, count: u32) {
    for n in 1..=count {
        assert_eq!(store::insert(conn, &entry(n), T0).unwrap(), Inserted::New);
    }
}

/// A random source that always hands out this word (so the pick over a pool of k is word % k).
fn word(w: u64) -> impl FnMut() -> Result<u64, String> {
    move || Ok(w)
}

fn picked(d: Drawn) -> (i64, i64, i64) {
    match d {
        Drawn::Picked { draw, winner } => {
            assert_eq!(winner.id, draw.winner_id);
            assert_eq!(winner.won, Some(draw.n), "the winner row already knows its draw number");
            (draw.n, draw.pool, draw.winner_id)
        }
        other => panic!("expected a winner, got {other:?}"),
    }
}

#[test]
fn a_new_database_is_private_empty_and_open() {
    let (dir, conn) = db();
    let db_file = dir.path().join("entries.sqlite3");
    assert_eq!(std::fs::metadata(&db_file).unwrap().permissions().mode() & 0o777, 0o600);
    assert!(store::is_open(&conn).unwrap(), "entries start open");
    let snap = store::snapshot(&conn).unwrap();
    assert_eq!((snap.rows.len(), snap.draws.len(), snap.eligible, snap.open), (0, 0, 0, true));
    fill(&conn, 1);
    for side in ["entries.sqlite3-wal", "entries.sqlite3-shm"] {
        if let Ok(m) = std::fs::metadata(dir.path().join(side)) {
            assert_eq!(m.permissions().mode() & 0o077, 0, "{side} is private too");
        }
    }
    let secure: i64 = conn.query_row("PRAGMA secure_delete", [], |r| r.get(0)).unwrap();
    assert_eq!(secure, 1, "deleted rows are zeroed in the file");
}

#[test]
fn reopening_keeps_the_entries_and_the_open_flag() {
    let dir = tempfile::tempdir().unwrap();
    {
        let conn = store::open(dir.path()).unwrap();
        fill(&conn, 2);
        store::set_open(&conn, false).unwrap();
    }
    let conn = store::open(dir.path()).unwrap();
    assert_eq!(store::list(&conn).unwrap().len(), 2);
    assert!(!store::is_open(&conn).unwrap());
    store::set_open(&conn, true).unwrap();
    assert!(store::is_open(&conn).unwrap());
}

#[test]
fn the_first_entry_per_email_wins_new_retry_taken() {
    let (_dir, conn) = db();
    assert_eq!(store::insert(&conn, &entry(1), T0).unwrap(), Inserted::New);
    assert_eq!(store::insert(&conn, &entry(2), T0).unwrap(), Inserted::New);
    let before = store::list(&conn).unwrap();
    // The phone resends the same entry after a lost answer.
    assert_eq!(store::insert(&conn, &entry(1), "2026-10-28T10:05:00Z").unwrap(), Inserted::Retry);
    // Another phone, same e-mail (any case): the first entry stays.
    let other = entry_with(9, 9, "TEST1@Example.COM", "Cits stāsts, ko otrs cilvēks gribēja pievienot.");
    assert_eq!(store::insert(&conn, &other, "2026-10-28T10:06:00Z").unwrap(), Inserted::Taken);
    // Same token with a corrected e-mail: still the phone's own entry, nothing changes.
    assert_eq!(store::insert(&conn, &entry_with(1, 1, "fixed@example.com", "Labots stāsts, kas nedrīkst nonākt datubāzē."), T0).unwrap(), Inserted::Retry);
    // A token that is stored AND an e-mail that belongs to another entry: the phone's own entry wins the answer.
    assert_eq!(store::insert(&conn, &entry_with(1, 1, "test2@example.com", "Vēl viens stāsts, kas nedrīkst nonākt datubāzē."), T0).unwrap(), Inserted::Retry);
    assert_eq!(store::list(&conn).unwrap(), before, "retry and taken never change a row");
}

#[test]
fn rows_come_back_in_submission_order_exactly_as_stored() {
    let (_dir, conn) = db();
    let story = "Rīga, \"Ķekava\" & <Jūrmala> 😀\nOtrā rindiņa ar 'pēdiņām'.";
    assert_eq!(store::insert(&conn, &entry_with(2, 2, "b@example.com", story), "2026-10-28T10:00:01Z").unwrap(), Inserted::New);
    assert_eq!(store::insert(&conn, &entry(3), "2026-10-28T10:00:02Z").unwrap(), Inserted::New);
    let rows = store::list(&conn).unwrap();
    assert_eq!(rows.iter().map(|r| r.id).collect::<Vec<_>>(), [1, 2]);
    let r = &rows[0];
    assert_eq!((r.at.as_str(), r.lang.as_str(), r.category.as_str(), r.story.as_str()), ("2026-10-28T10:00:01Z", "lv", "trip", story));
    assert_eq!((r.first.as_str(), r.last.as_str(), r.email.as_str()), ("Testa2", "Ķirsis", "b@example.com"));
    assert_eq!((r.phone.as_str(), r.company.as_str()), ("+371 20000002", "Parauga SIA"));
    assert_eq!((r.share, r.excluded, r.won, r.dup_of), (true, false, None, None));
    assert!(!rows[1].share, "the share flag is kept per entry");
    assert_eq!(store::get(&conn, 2).unwrap().unwrap(), rows[1]);
    assert_eq!(store::get(&conn, 3).unwrap(), None);
}

#[test]
fn exclude_sets_lifts_and_reports_a_missing_entry() {
    let (_dir, conn) = db();
    fill(&conn, 2);
    assert!(store::exclude(&conn, 1, true).unwrap());
    assert!(store::exclude(&conn, 1, true).unwrap(), "setting it twice is fine");
    assert!(store::list(&conn).unwrap()[0].excluded);
    assert_eq!(store::snapshot(&conn).unwrap().eligible, 1);
    assert!(store::exclude(&conn, 1, false).unwrap());
    assert!(!store::list(&conn).unwrap()[0].excluded);
    assert!(!store::exclude(&conn, 99, true).unwrap(), "no such entry");
}

#[test]
fn the_draw_skips_excluded_entries_and_earlier_winners_and_numbers_from_one() {
    let (_dir, mut conn) = db();
    fill(&conn, 5);
    store::exclude(&conn, 2, true).unwrap();
    store::set_open(&conn, false).unwrap();
    // Always the first of the pool: 1, then 3, 4, 5 (2 is excluded, earlier winners drop out).
    let drawn: Vec<(i64, i64, i64)> = (0..4).map(|i| picked(store::draw(&mut conn, &format!("2026-10-28T11:0{i}:00Z"), &mut word(0)).unwrap())).collect();
    assert_eq!(drawn, [(1, 4, 1), (2, 3, 3), (3, 2, 4), (4, 1, 5)], "(draw number, pool, winner id)");
    assert_eq!(store::draw(&mut conn, T0, &mut word(0)).unwrap(), Drawn::Empty);
    assert_eq!(store::draws(&conn).unwrap().len(), 4, "a refused draw leaves no row");
    let won: Vec<Option<i64>> = store::list(&conn).unwrap().iter().map(|r| r.won).collect();
    assert_eq!(won, [Some(1), None, Some(2), Some(3), Some(4)]);
    let draws = store::draws(&conn).unwrap();
    assert_eq!((draws[0].at.as_str(), draws[3].at.as_str()), ("2026-10-28T11:00:00Z", "2026-10-28T11:03:00Z"));
    assert_eq!(store::snapshot(&conn).unwrap().eligible, 0);
}

#[test]
fn the_pick_is_the_sources_index_into_the_eligible_ids() {
    let (_dir, mut conn) = db();
    fill(&conn, 5);
    store::exclude(&conn, 1, true).unwrap();
    store::set_open(&conn, false).unwrap();
    // Pool [2, 3, 4, 5]: word 3 is index 3, so entry 5; then pool [2, 3, 4]: word 1 is entry 3.
    assert_eq!(picked(store::draw(&mut conn, T0, &mut word(3)).unwrap()), (1, 4, 5));
    assert_eq!(picked(store::draw(&mut conn, T0, &mut word(1)).unwrap()), (2, 3, 3));
    // Words past the pool wrap around the pool, they never leave it: pool [2, 4], word 5 is index 1.
    assert_eq!(picked(store::draw(&mut conn, T0, &mut word(5)).unwrap()), (3, 2, 4));
}

#[test]
fn a_failing_random_source_leaves_no_trace() {
    let (_dir, mut conn) = db();
    fill(&conn, 3);
    store::set_open(&conn, false).unwrap();
    let err = store::draw(&mut conn, T0, &mut || Err("urandom is gone".to_string())).unwrap_err();
    assert!(err.starts_with("store: draw:") && err.contains("urandom is gone"), "{err}");
    assert!(store::draws(&conn).unwrap().is_empty());
    assert_eq!(store::snapshot(&conn).unwrap().eligible, 3);
    assert_eq!(picked(store::draw(&mut conn, T0, &mut word(0)).unwrap()), (1, 3, 1), "the next draw is still number 1");
}

#[test]
fn nobody_eligible_asks_for_no_randomness_and_writes_nothing() {
    let (_dir, mut conn) = db();
    let mut calls = 0;
    let mut source = || {
        calls += 1;
        Ok(0)
    };
    store::set_open(&conn, false).unwrap();
    assert_eq!(store::draw(&mut conn, T0, &mut source).unwrap(), Drawn::Empty, "no entries at all");
    fill(&conn, 2);
    store::exclude(&conn, 1, true).unwrap();
    store::exclude(&conn, 2, true).unwrap();
    assert_eq!(store::draw(&mut conn, T0, &mut source).unwrap(), Drawn::Empty, "all excluded");
    assert_eq!(calls, 0);
    assert!(store::draws(&conn).unwrap().is_empty());
}

#[test]
fn clear_refuses_while_open_then_deletes_every_entry_and_draw() {
    let (_dir, mut conn) = db();
    fill(&conn, 3);
    store::set_open(&conn, false).unwrap();
    picked(store::draw(&mut conn, T0, &mut word(0)).unwrap());
    store::set_open(&conn, true).unwrap();
    assert_eq!(store::clear(&mut conn).unwrap(), Cleared::StillOpen);
    assert_eq!((store::list(&conn).unwrap().len(), store::draws(&conn).unwrap().len()), (3, 1), "nothing deleted while open");
    store::set_open(&conn, false).unwrap();
    assert_eq!(store::clear(&mut conn).unwrap(), Cleared::Deleted(3));
    assert!(store::list(&conn).unwrap().is_empty() && store::draws(&conn).unwrap().is_empty());
    assert!(!store::is_open(&conn).unwrap(), "clearing does not reopen entries");
    assert_eq!(store::clear(&mut conn).unwrap(), Cleared::Deleted(0), "an empty closed database clears too");
}

#[test]
fn after_a_clear_numbering_starts_again_at_one() {
    let (_dir, mut conn) = db();
    fill(&conn, 2);
    store::set_open(&conn, false).unwrap();
    picked(store::draw(&mut conn, T0, &mut word(0)).unwrap());
    picked(store::draw(&mut conn, T0, &mut word(0)).unwrap());
    store::clear(&mut conn).unwrap();
    store::set_open(&conn, true).unwrap();
    fill(&conn, 1);
    assert_eq!(store::list(&conn).unwrap()[0].id, 1);
    store::set_open(&conn, false).unwrap();
    assert_eq!(picked(store::draw(&mut conn, T0, &mut word(0)).unwrap()), (1, 1, 1));
}

#[test]
fn cleared_personal_data_is_gone_from_the_file_and_its_log() {
    let dir = tempfile::tempdir().unwrap();
    let marker = "ZQXJ-marker-story-that-must-not-survive-a-clear";
    {
        let mut conn = store::open(dir.path()).unwrap();
        store::insert(&conn, &entry_with(1, 1, "marker-zqxj@example.com", marker), T0).unwrap();
        assert!(String::from_utf8_lossy(&std::fs::read(dir.path().join("entries.sqlite3-wal")).unwrap_or_default()).contains(marker), "it was written");
        store::set_open(&conn, false).unwrap();
        assert_eq!(store::clear(&mut conn).unwrap(), Cleared::Deleted(1));
    }
    for file in ["entries.sqlite3", "entries.sqlite3-wal"] {
        let bytes = std::fs::read(dir.path().join(file)).unwrap_or_default();
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains(marker) && !text.contains("marker-zqxj"), "{file} still holds the cleared entry");
    }
}

#[test]
fn the_eligible_count_is_the_next_draws_pool() {
    let (_dir, mut conn) = db();
    fill(&conn, 6);
    store::exclude(&conn, 2, true).unwrap();
    store::exclude(&conn, 3, true).unwrap();
    store::set_open(&conn, false).unwrap();
    picked(store::draw(&mut conn, T0, &mut word(0)).unwrap());
    let snap = store::snapshot(&conn).unwrap();
    assert_eq!((snap.rows.len(), snap.eligible), (6, 3));
    assert_eq!(picked(store::draw(&mut conn, T0, &mut word(0)).unwrap()).1, 3);
}

#[test]
fn a_draw_waits_until_entries_are_closed() {
    let (_dir, mut conn) = db();
    fill(&conn, 2);
    assert_eq!(store::draw(&mut conn, T0, &mut word(0)).unwrap(), Drawn::StillOpen);
    assert!(store::draws(&conn).unwrap().is_empty(), "a refused draw leaves no row");
    store::set_open(&conn, false).unwrap();
    assert_eq!(picked(store::draw(&mut conn, T0, &mut word(0)).unwrap()), (1, 2, 1));
}

#[test]
fn a_draw_keeps_the_ids_it_drew_from() {
    let (_dir, mut conn) = db();
    fill(&conn, 3);
    store::exclude(&conn, 2, true).unwrap();
    store::set_open(&conn, false).unwrap();
    picked(store::draw(&mut conn, T0, &mut word(0)).unwrap());
    store::exclude(&conn, 3, true).unwrap();
    assert_eq!(store::draws(&conn).unwrap()[0].pool_ids, [1, 3], "later exclusions do not rewrite the pool");
}

#[test]
fn the_same_phone_under_another_email_starts_out_of_the_draw() {
    let (_dir, conn) = db();
    let story = "Satiku jaunu cilvēku pasākumā un mēs runājām.";
    assert_eq!(store::insert(&conn, &entry_full(1, 1, "a@example.com", story, "+371 20 000 001"), T0).unwrap(), Inserted::New);
    // The same number without the country code and spaced differently is the same person.
    assert_eq!(store::insert(&conn, &entry_full(2, 2, "b@example.com", story, "2000-0001"), T0).unwrap(), Inserted::SamePhone(1));
    assert_eq!(store::insert(&conn, &entry_full(3, 3, "c@example.com", story, "+371 20 000 003"), T0).unwrap(), Inserted::New);
    // Its own resend is still a retry, not a second copy.
    assert_eq!(store::insert(&conn, &entry_full(2, 2, "b@example.com", story, "2000-0001"), T0).unwrap(), Inserted::Retry);
    let rows = store::list(&conn).unwrap();
    let flags: Vec<(bool, Option<i64>)> = rows.iter().map(|r| (r.excluded, r.dup_of)).collect();
    assert_eq!(flags, [(false, None), (true, Some(1)), (false, None)]);
    assert_eq!(store::snapshot(&conn).unwrap().eligible, 2);
    // The organisers can put it back.
    store::exclude(&conn, 2, false).unwrap();
    assert_eq!(store::snapshot(&conn).unwrap().eligible, 3);
}

#[test]
fn delete_removes_one_entry_but_never_a_winner() {
    let (_dir, mut conn) = db();
    fill(&conn, 3);
    store::set_open(&conn, false).unwrap();
    picked(store::draw(&mut conn, T0, &mut word(0)).unwrap());
    assert_eq!(store::delete(&mut conn, 1).unwrap(), Deleted::Winner);
    assert_eq!(store::delete(&mut conn, 2).unwrap(), Deleted::Gone);
    assert_eq!(store::delete(&mut conn, 2).unwrap(), Deleted::Missing);
    assert_eq!(store::delete(&mut conn, 99).unwrap(), Deleted::Missing);
    let ids: Vec<i64> = store::list(&conn).unwrap().iter().map(|r| r.id).collect();
    assert_eq!(ids, [1, 3]);
}

#[test]
fn a_deleted_entry_is_gone_from_the_file_and_its_log() {
    let dir = tempfile::tempdir().unwrap();
    let marker = "ZQXK-marker-story-that-must-not-survive-a-delete";
    {
        let mut conn = store::open(dir.path()).unwrap();
        store::insert(&conn, &entry_with(1, 1, "keep@example.com", "Satiku jaunu cilvēku pasākumā un mēs runājām."), T0).unwrap();
        store::insert(&conn, &entry_with(2, 2, "marker-zqxk@example.com", marker), T0).unwrap();
        assert_eq!(store::delete(&mut conn, 2).unwrap(), Deleted::Gone);
    }
    for file in ["entries.sqlite3", "entries.sqlite3-wal"] {
        let bytes = std::fs::read(dir.path().join(file)).unwrap_or_default();
        let text = String::from_utf8_lossy(&bytes);
        assert!(!text.contains(marker) && !text.contains("marker-zqxk"), "{file} still holds the deleted entry");
    }
}

#[test]
fn the_same_phone_check_reads_an_index_not_every_row() {
    let (_dir, conn) = db();
    for sql in [
        "SELECT MIN(id) FROM entries WHERE phone_key = ?1 AND ?1 <> ''",
        "SELECT EXISTS (SELECT 1 FROM entries WHERE phone_key = ?1 AND ?1 <> '')",
    ] {
        let mut stmt = conn.prepare(&format!("EXPLAIN QUERY PLAN {sql}")).unwrap();
        let plan: Vec<String> =
            stmt.query_map(["+37120000001"], |r| r.get(3)).unwrap().map(Result::unwrap).collect();
        assert!(plan.iter().any(|step| step.contains("entries_phone")), "{sql}: {plan:?}");
        assert!(!plan.iter().any(|step| step.starts_with("SCAN entries")), "{sql}: {plan:?}");
    }
}
