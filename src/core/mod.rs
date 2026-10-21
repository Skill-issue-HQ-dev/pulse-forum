//! Pure logic: the entry rules, the draw's pick, the admin key check, labels, Riga time and the workbook.
//! Owns no file, network, clock or process I/O; the io and web layers do that.

pub mod auth;
pub mod draw;
pub mod entry;
pub mod labels;
pub mod record;
pub mod riga;
pub mod stamp;
pub mod workbook;
pub mod xlsx;
