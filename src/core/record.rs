//! A stored entry and a draw row as the store returns them and the admin API and the export show them.
//! Owns the two record shapes and their JSON field names (the contract's `entries` and `draws` items).
//! Invariant: times are RFC 3339 UTC strings; `won` is the draw number that picked the entry, else null.

use serde::Serialize;

/// One stored entry (a row of the admin list and of the `Stāsti` sheet).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Row {
    pub id: i64,
    pub at: String,
    pub lang: String,
    pub category: String,
    pub story: String,
    pub first: String,
    pub last: String,
    pub email: String,
    pub phone: String,
    pub company: String,
    pub share: bool,
    pub excluded: bool,
    pub won: Option<i64>,
}

/// One draw: its number from 1, when it ran, how many entries were eligible, and who it picked.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Draw {
    pub n: i64,
    pub at: String,
    pub pool: i64,
    pub winner_id: i64,
}
