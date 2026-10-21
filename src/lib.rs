//! pulse-forum: the collector behind the PULSE Forum 2026 story draw (entries in, admin draw, Excel out).
//! Layers: cli (src/main.rs) -> web (HTTP edge) -> core (pure rules, draw, workbook) -> io (SQLite, clock, randomness).
//! Invariant: nothing is stored unless core::entry accepted it, and a draw's pick and its row are one transaction.

pub mod config;
pub mod core;
pub mod io;
pub mod web;
