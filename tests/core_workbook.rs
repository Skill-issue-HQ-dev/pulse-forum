//! Tests for core::workbook and core::riga: a real .xlsx package with three sheets in the contract's columns,
//! escaped text (emoji kept), a text-format phone, and Riga wall-clock times across the October clock change.

use pulse_forum::core::record::{Draw, Row};
use pulse_forum::core::riga::{excel_serial, file_stamp};
use pulse_forum::core::workbook::{build, file_name, Facts, DRAW_HEADERS, STORY_HEADERS};
use pulse_forum::core::xlsx::escape;
use std::io::{Cursor, Read};

fn archive(bytes: &[u8]) -> zip::ZipArchive<Cursor<Vec<u8>>> {
    zip::ZipArchive::new(Cursor::new(bytes.to_vec())).expect("the workbook is a zip")
}

fn part(bytes: &[u8], name: &str) -> String {
    let mut s = String::new();
    archive(bytes).by_name(name).unwrap_or_else(|_| panic!("no part {name}")).read_to_string(&mut s).expect("utf-8 xml");
    s
}

/// The `<c ...>...</c>` text of one cell, by its reference ("I2").
fn cell<'a>(xml: &'a str, at: &str) -> &'a str {
    let start = xml.find(&format!("<c r=\"{at}\"")).unwrap_or_else(|| panic!("no cell {at}"));
    let end = xml[start..].find("</c>").expect("cell is closed") + start + 4;
    &xml[start..end]
}

fn has_cell(xml: &str, at: &str) -> bool {
    xml.contains(&format!("<c r=\"{at}\""))
}

fn row(id: i64, first: &str) -> Row {
    Row {
        id,
        at: "2026-10-28T10:15:02Z".into(),
        lang: "lv".into(),
        category: "new_contact".into(),
        story: "Satiku jaunu cilvēku.".into(),
        first: first.into(),
        last: "Ķirsis".into(),
        email: "test@example.com".into(),
        phone: "+371 20 000 000".into(),
        company: "Parauga SIA".into(),
        share: true,
        excluded: false,
        won: None,
    }
}

fn facts(open: bool) -> Facts {
    Facts { exported_at: "2026-10-28T10:20:00Z".into(), open }
}

#[test]
fn the_package_has_every_part_and_the_three_sheets_in_order() {
    let bytes = build(&[row(1, "Testa")], &[], &facts(true)).unwrap();
    assert!(bytes.starts_with(b"PK"));
    let mut zip = archive(&bytes);
    let mut names: Vec<String> = (0..zip.len()).map(|i| zip.by_index(i).unwrap().name().unwrap().into_owned()).collect();
    names.sort();
    let mut want = vec![
        "[Content_Types].xml", "_rels/.rels", "xl/workbook.xml", "xl/_rels/workbook.xml.rels", "xl/styles.xml",
        "xl/worksheets/sheet1.xml", "xl/worksheets/sheet2.xml", "xl/worksheets/sheet3.xml",
    ];
    want.sort_unstable();
    assert_eq!(names, want);
    let book = part(&bytes, "xl/workbook.xml");
    let at = |name: &str| book.find(&format!("name=\"{name}\"")).unwrap_or_else(|| panic!("no sheet {name}"));
    assert!(at("Stāsti") < at("Izlozes") && at("Izlozes") < at("Eksports"));
    let types = part(&bytes, "[Content_Types].xml");
    for n in 1..=3 {
        assert!(types.contains(&format!("/xl/worksheets/sheet{n}.xml")), "sheet {n} is declared");
    }
}

#[test]
fn the_stories_sheet_has_the_contracts_columns_and_one_row_per_entry() {
    let mut rows = vec![row(1, "Testa"), row(2, "Ābele"), row(3, "Zaķis")];
    rows[1].won = Some(1);
    rows[2].excluded = true;
    rows[2].share = false;
    rows[2].lang = "en".into();
    rows[2].category = "trip".into();
    let bytes = build(&rows, &[], &facts(true)).unwrap();
    let s1 = part(&bytes, "xl/worksheets/sheet1.xml");
    assert_eq!(s1.matches("<row ").count(), 4, "header + 3 entries");
    assert_eq!(STORY_HEADERS.len(), 13);
    for (i, h) in STORY_HEADERS.iter().enumerate() {
        let at = format!("{}1", char::from(b'A' + i as u8));
        assert!(cell(&s1, &at).contains(&format!("<t>{h}</t>")), "{at} is {h}");
    }
    assert_eq!(
        STORY_HEADERS,
        ["Nr.", "Iesniegts Rīgas laikā", "Valoda", "Kategorija", "Stāsts", "Vārds", "Uzvārds", "E-pasts", "Tālrunis", "Uzņēmums", "Drīkst publicēt stāstu", "Izslēgts no izlozes", "Uzvarētājs izlozē Nr."]
    );
    // Entry 1: number, language, the Latvian category label, the names, JĀ/NĒ, no draw number.
    assert!(cell(&s1, "A2").contains("<v>1</v>"));
    assert!(cell(&s1, "C2").contains("<t>LV</t>") && cell(&s1, "D2").contains("<t>Jauns kontakts</t>"));
    assert!(cell(&s1, "F2").contains("<t>Testa</t>") && cell(&s1, "G2").contains("<t>Ķirsis</t>"));
    assert!(cell(&s1, "K2").contains("<t>JĀ</t>") && cell(&s1, "L2").contains("<t>NĒ</t>"));
    assert!(!has_cell(&s1, "M2"), "an entry that won nothing has no draw number");
    // Entry 2 won draw 1; entry 3 is excluded, English, not for publication, with the trip label.
    assert!(cell(&s1, "M3").contains("<v>1</v>"));
    assert!(cell(&s1, "C4").contains("<t>EN</t>") && cell(&s1, "D4").contains("<t>Ceļojums</t>"));
    assert!(cell(&s1, "K4").contains("<t>NĒ</t>") && cell(&s1, "L4").contains("<t>JĀ</t>"));
}

#[test]
fn special_characters_are_escaped_and_emoji_survive() {
    let mut r = row(1, "Testa");
    r.story = "Tom & Jerry <b>\"hi\"</b> 'x' 😀 Rīga\nsecond line".into();
    r.company = "A & B <SIA>".into();
    r.email = "a\u{1}b@example.com".into();
    let bytes = build(&[r], &[], &facts(true)).unwrap();
    let s1 = part(&bytes, "xl/worksheets/sheet1.xml");
    assert!(s1.contains("<t>Tom &amp; Jerry &lt;b&gt;&quot;hi&quot;&lt;/b&gt; &apos;x&apos; 😀 Rīga\nsecond line</t>"), "{s1}");
    assert!(s1.contains("<t>A &amp; B &lt;SIA&gt;</t>"));
    assert!(s1.contains("<t>ab@example.com</t>"), "a control character XML forbids is dropped");
    assert!(!s1.contains("<b>") && !s1.contains("\u{1}"));
    assert_eq!(escape("& < > \" '"), "&amp; &lt; &gt; &quot; &apos;");
}

#[test]
fn a_phone_number_stays_a_text_cell() {
    let mut a = row(1, "Testa");
    a.phone = "+371 20 000 000".into();
    let mut b = row(2, "Ābele");
    b.phone = "0037120000000".into();
    let bytes = build(&[a, b], &[], &facts(true)).unwrap();
    let s1 = part(&bytes, "xl/worksheets/sheet1.xml");
    assert_eq!(cell(&s1, "I2"), "<c r=\"I2\" s=\"3\" t=\"inlineStr\"><is><t>+371 20 000 000</t></is></c>");
    assert_eq!(cell(&s1, "I3"), "<c r=\"I3\" s=\"3\" t=\"inlineStr\"><is><t>0037120000000</t></is></c>");
    assert!(part(&bytes, "xl/styles.xml").contains("numFmtId=\"49\""), "style 3 is the text format");
}

#[test]
fn times_are_riga_wall_clock_in_the_sheets() {
    let bytes = build(&[row(1, "Testa")], &[], &facts(true)).unwrap();
    // 10:15:02 UTC on 28.10.2026 is 12:15:02 in Riga: EET (UTC+2) since the clock change on 25.10.
    let s1 = part(&bytes, "xl/worksheets/sheet1.xml");
    assert_eq!(cell(&s1, "B2"), "<c r=\"B2\" s=\"2\"><v>46323.510440</v></c>");
    let s3 = part(&bytes, "xl/worksheets/sheet3.xml");
    assert!(cell(&s3, "B1").contains("<v>46323.513889</v>"), "10:20:00Z is 12:20 Riga");
    assert!(part(&bytes, "xl/styles.xml").contains("formatCode=\"dd.mm.yyyy hh:mm\""));
}

#[test]
fn riga_time_follows_the_october_clock_change() {
    let hours = |t: &str| excel_serial(t).unwrap().fract() * 24.0;
    assert!((hours("2026-10-28T10:15:02Z") - (12.0 + 15.0 / 60.0 + 2.0 / 3600.0)).abs() < 1e-6, "winter, UTC+2");
    assert!((hours("2026-10-24T10:00:00Z") - 13.0).abs() < 1e-6, "summer, UTC+3");
    // The night of 25.10.2026: at 01:00 UTC the clocks go from 04:00 back to 03:00, so 03:30 happens twice.
    assert!((hours("2026-10-25T00:30:00Z") - 3.5).abs() < 1e-6, "still summer time");
    assert!((hours("2026-10-25T01:30:00Z") - 3.5).abs() < 1e-6, "an hour later, winter time");
    assert_eq!(excel_serial("2026-10-28T10:15:02Z").unwrap().trunc() as i64, 46_323);
    assert_eq!(excel_serial("2026-11-17T08:00:00Z").unwrap().trunc() as i64, 46_343);
    assert_eq!(excel_serial("not a time"), None);
    assert_eq!(excel_serial(""), None);
}

#[test]
fn the_file_name_is_the_riga_stamp_in_ascii() {
    assert_eq!(file_stamp("2026-10-28T10:15:02Z").as_deref(), Some("2026-10-28 12-15"));
    assert_eq!(file_name("2026-10-28T10:15:02Z"), "PULSE izloze 2026-10-28 12-15.xlsx");
    assert_eq!(file_name("2026-07-01T21:30:00Z"), "PULSE izloze 2026-07-02 00-30.xlsx", "summer, and past midnight");
    assert!(file_name("2026-10-28T10:15:02Z").is_ascii());
    assert_eq!(file_name("garbage"), "PULSE izloze.xlsx");
}

#[test]
fn the_draws_sheet_names_each_winner_by_entry_number_and_name() {
    let mut rows = vec![row(1, "Testa"), row(2, "Ābele"), row(3, "Zaķis")];
    rows[1].won = Some(1);
    let draws = vec![
        Draw { n: 1, at: "2026-10-28T10:30:00Z".into(), pool: 3, winner_id: 2 },
        Draw { n: 2, at: "2026-10-28T10:31:00Z".into(), pool: 2, winner_id: 9 },
    ];
    let bytes = build(&rows, &draws, &facts(false)).unwrap();
    let s2 = part(&bytes, "xl/worksheets/sheet2.xml");
    assert_eq!(s2.matches("<row ").count(), 3, "header + 2 draws");
    assert_eq!(DRAW_HEADERS, ["Nr.", "Laiks Rīgā", "Dalībnieku skaits izlozē", "Uzvarētājs"]);
    for (i, h) in DRAW_HEADERS.iter().enumerate() {
        let at = format!("{}1", char::from(b'A' + i as u8));
        assert!(cell(&s2, &at).contains(&format!("<t>{h}</t>")), "{at} is {h}");
    }
    assert!(cell(&s2, "A2").contains("<v>1</v>") && cell(&s2, "C2").contains("<v>3</v>"));
    assert!(cell(&s2, "B2").contains("<v>46323.520833</v>"), "10:30Z is 12:30 Riga");
    assert!(cell(&s2, "D2").contains("<t>Nr. 2: Ābele Ķirsis</t>"));
    assert!(cell(&s2, "D3").contains("<t>Nr. 9</t>"), "a winner no longer in the list still shows its number");
}

#[test]
fn the_export_sheet_says_when_how_many_and_open_or_closed() {
    let rows = [row(1, "Testa"), row(2, "Ābele")];
    let open = part(&build(&rows, &[], &facts(true)).unwrap(), "xl/worksheets/sheet3.xml");
    assert!(cell(&open, "A1").contains("Eksportēts") && has_cell(&open, "B1"));
    assert!(cell(&open, "A2").contains("<t>Stāstu skaits</t>") && cell(&open, "B2").contains("<v>2</v>"));
    assert!(cell(&open, "A3").contains("<t>Stāstu pieņemšana</t>") && cell(&open, "B3").contains("<t>Atvērta</t>"));
    let closed = part(&build(&rows, &[], &facts(false)).unwrap(), "xl/worksheets/sheet3.xml");
    assert!(cell(&closed, "B3").contains("<t>Slēgta</t>"));
}

#[test]
fn an_empty_draw_still_gives_a_workbook_with_header_rows() {
    let bytes = build(&[], &[], &facts(true)).unwrap();
    assert_eq!(part(&bytes, "xl/worksheets/sheet1.xml").matches("<row ").count(), 1);
    assert_eq!(part(&bytes, "xl/worksheets/sheet2.xml").matches("<row ").count(), 1);
    assert!(cell(&part(&bytes, "xl/worksheets/sheet3.xml"), "B2").contains("<v>0</v>"));
}

#[test]
fn the_header_row_is_frozen_on_the_two_data_sheets() {
    let bytes = build(&[row(1, "Testa")], &[], &facts(true)).unwrap();
    for n in [1, 2] {
        assert!(part(&bytes, &format!("xl/worksheets/sheet{n}.xml")).contains("state=\"frozen\""), "sheet {n}");
    }
    assert!(!part(&bytes, "xl/worksheets/sheet3.xml").contains("frozen"));
}
