//! A stored entry and a draw row as the store returns them and the admin API and the export show them.
//! Owns the two record shapes and their JSON field names (the contract's `entries` and `draws` items).
//! Invariant: times are RFC 3339 UTC strings; `won` is the draw number that picked the entry, else null;
//! `dup_of` is the earlier entry with the same phone, else null.

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
    /// The earlier entry with the same phone: this one started out of the draw [one person, one chance].
    pub dup_of: Option<i64>,
}

/// One draw: its number from 1, when it ran, how many entries were eligible, who it picked, and the ids of
/// every entry it drew from [so the pool can be checked later, whatever was excluded afterwards].
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Draw {
    pub n: i64,
    pub at: String,
    pub pool: i64,
    pub winner_id: i64,
    pub pool_ids: Vec<i64>,
}
