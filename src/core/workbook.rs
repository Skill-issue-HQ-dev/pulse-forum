//! The Excel workbook the organiser downloads, in Latvian: `Stāsti` (one row per entry), `Izlozes` (one row per
//! draw) and `Eksports` (when, how many, open or closed). Owns the sheet layout, column names and file name.
//! Invariant: one row per stored entry and per draw, in order; a phone stays text; times are Riga wall-clock.

use crate::config;
use crate::core::record::{Draw, Row};
use crate::core::xlsx::{self, Cell, Sheet};
use crate::core::{labels, riga};

pub const STORIES_SHEET: &str = "Stāsti";
pub const DRAWS_SHEET: &str = "Izlozes";
pub const EXPORT_SHEET: &str = "Eksports";

pub const STORY_HEADERS: [&str; 13] = [
    "Nr.",
    "Iesniegts Rīgas laikā",
    "Valoda",
    "Kategorija",
    "Stāsts",
    "Vārds",
    "Uzvārds",
    "E-pasts",
    "Tālrunis",
    "Uzņēmums",
    "Drīkst publicēt stāstu",
    "Izslēgts no izlozes",
    "Uzvarētājs izlozē Nr.",
];
pub const DRAW_HEADERS: [&str; 4] = ["Nr.", "Laiks Rīgā", "Dalībnieku skaits izlozē", "Uzvarētājs"];

const STORY_WIDTHS: [u8; 13] = [6, 20, 9, 24, 70, 16, 18, 30, 18, 24, 16, 16, 14];
const DRAW_WIDTHS: [u8; 4] = [6, 20, 24, 40];
const EXPORT_WIDTHS: [u8; 2] = [28, 20];

/// What the `Eksports` sheet says besides the data: when the file was made and whether entries are open.
pub struct Facts {
    pub exported_at: String,
    pub open: bool,
}

/// The workbook as .xlsx bytes.
pub fn build(rows: &[Row], draws: &[Draw], facts: &Facts) -> Result<Vec<u8>, String> {
    xlsx::package(&[stories_sheet(rows), draws_sheet(rows, draws), export_sheet(rows.len(), facts)])
}

/// The download's file name: "PULSE izloze 2026-10-28 12-15.xlsx" (Riga wall clock, ASCII).
pub fn file_name(exported_at: &str) -> String {
    match riga::file_stamp(exported_at) {
        Some(stamp) => format!("{} {stamp}.xlsx", config::EXPORT_STEM),
        None => format!("{}.xlsx", config::EXPORT_STEM),
    }
}

fn header_row(names: &[&str]) -> Vec<Cell> {
    names.iter().map(|h| Cell::text(*h, xlsx::HEAD)).collect()
}

fn stories_sheet(rows: &[Row]) -> Sheet {
    let mut out = vec![header_row(&STORY_HEADERS)];
    out.extend(rows.iter().map(story_row));
    Sheet { name: STORIES_SHEET, widths: STORY_WIDTHS.to_vec(), freeze_header: true, rows: out }
}

fn story_row(r: &Row) -> Vec<Cell> {
    let category = labels::category(&r.category).map_or(r.category.as_str(), |c| c.lv);
    vec![
        Cell::Num(r.id as f64, xlsx::TOP),
        Cell::date(riga::excel_serial(&r.at)),
        Cell::text(r.lang.to_uppercase(), xlsx::TOP),
        Cell::text(category, xlsx::TOP),
        Cell::text(&r.story, xlsx::WRAP),
        Cell::text(&r.first, xlsx::TOP),
        Cell::text(&r.last, xlsx::TOP),
        Cell::text(&r.email, xlsx::TOP),
        Cell::text(&r.phone, xlsx::PHONE),
        Cell::text(&r.company, xlsx::TOP),
        Cell::text(yes_no(r.share), xlsx::TOP),
        Cell::text(yes_no(r.excluded), xlsx::TOP),
        r.won.map_or(Cell::Empty, |n| Cell::Num(n as f64, xlsx::TOP)),
    ]
}

fn draws_sheet(rows: &[Row], draws: &[Draw]) -> Sheet {
    let mut out = vec![header_row(&DRAW_HEADERS)];
    out.extend(draws.iter().map(|d| draw_row(rows, d)));
    Sheet { name: DRAWS_SHEET, widths: DRAW_WIDTHS.to_vec(), freeze_header: true, rows: out }
}

fn draw_row(rows: &[Row], d: &Draw) -> Vec<Cell> {
    vec![
        Cell::Num(d.n as f64, xlsx::TOP),
        Cell::date(riga::excel_serial(&d.at)),
        Cell::Num(d.pool as f64, xlsx::TOP),
        Cell::text(winner_label(rows, d), xlsx::TOP),
    ]
}

/// "Nr. 5: First Last": the entry number and the name, as the organiser reads them on the stage.
fn winner_label(rows: &[Row], d: &Draw) -> String {
    match rows.iter().find(|r| r.id == d.winner_id) {
        Some(r) => format!("Nr. {}: {} {}", r.id, r.first, r.last),
        None => format!("Nr. {}", d.winner_id),
    }
}

fn export_sheet(count: usize, facts: &Facts) -> Sheet {
    let label = |s: &str| Cell::text(s, xlsx::PLAIN);
    let intake = if facts.open { "Atvērta" } else { "Slēgta" };
    Sheet {
        name: EXPORT_SHEET,
        widths: EXPORT_WIDTHS.to_vec(),
        freeze_header: false,
        rows: vec![
            vec![label("Eksportēts (Rīgas laiks)"), Cell::date(riga::excel_serial(&facts.exported_at))],
            vec![label("Stāstu skaits"), Cell::Num(count as f64, xlsx::PLAIN)],
            vec![label("Stāstu pieņemšana"), label(intake)],
        ],
    }
}

fn yes_no(flag: bool) -> &'static str {
    if flag {
        "JĀ"
    } else {
        "NĒ"
    }
}
