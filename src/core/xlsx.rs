//! The .xlsx package as plain SpreadsheetML in a zip (rust_xlsxwriter cannot be used: every zip crate version
//! it accepts was yanked in 2026). Owns the cell and sheet model, XML escaping and the static package parts.
//! Invariant: text is always an inline string, so a phone number or a story is never read as a number or a formula.

use std::fmt::Write as _;
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

/// Style ids of xl/styles.xml: default look, header (bold white on dark, wrapped), Riga date-time
/// "dd.mm.yyyy hh:mm", text format for phones, wrapped text for stories, plain top-aligned.
pub const PLAIN: u8 = 0;
pub const HEAD: u8 = 1;
pub const DATE: u8 = 2;
pub const PHONE: u8 = 3;
pub const WRAP: u8 = 4;
pub const TOP: u8 = 5;

/// One cell of a row: text, a number, a Riga date-time (an Excel serial number), or nothing.
pub enum Cell {
    Text(String, u8),
    Num(f64, u8),
    Date(f64),
    Empty,
}

impl Cell {
    pub fn text(s: impl Into<String>, style: u8) -> Cell {
        Cell::Text(s.into(), style)
    }
    /// A date-time from an optional Excel serial; None leaves the cell empty.
    pub fn date(serial: Option<f64>) -> Cell {
        serial.map_or(Cell::Empty, Cell::Date)
    }
}

/// One worksheet: its tab name, column widths, whether the header row is frozen, and the rows.
pub struct Sheet {
    pub name: &'static str,
    pub widths: Vec<u8>,
    pub freeze_header: bool,
    pub rows: Vec<Vec<Cell>>,
}

const NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
const REL_NS: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
const DECL: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n";

/// The workbook as .xlsx bytes: the given sheets in order, one shared style sheet.
pub fn package(sheets: &[Sheet]) -> Result<Vec<u8>, String> {
    let mut parts: Vec<(String, String)> = vec![
        ("[Content_Types].xml".into(), content_types(sheets.len())),
        ("_rels/.rels".into(), ROOT_RELS.into()),
        ("xl/workbook.xml".into(), workbook_xml(sheets)),
        ("xl/_rels/workbook.xml.rels".into(), workbook_rels(sheets.len())),
        ("xl/styles.xml".into(), STYLES.into()),
    ];
    for (i, s) in sheets.iter().enumerate() {
        parts.push((format!("xl/worksheets/sheet{}.xml", i + 1), sheet_xml(s)));
    }
    zip_parts(&parts).map_err(|e| format!("workbook: zip: {e}"))
}

fn zip_parts(parts: &[(String, String)]) -> zip::result::ZipResult<Vec<u8>> {
    let mut zw = ZipWriter::new(Cursor::new(Vec::new()));
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
    for (name, xml) in parts {
        zw.start_file(name.as_str(), opts)?;
        zw.write_all(xml.as_bytes())?;
    }
    Ok(zw.finish()?.into_inner())
}

fn content_types(sheets: usize) -> String {
    let mut x = format!("{DECL}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/>");
    for n in 1..=sheets {
        let _ = write!(x, "<Override PartName=\"/xl/worksheets/sheet{n}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>");
    }
    x.push_str("<Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/></Types>");
    x
}

fn workbook_xml(sheets: &[Sheet]) -> String {
    let mut x = format!("{DECL}<workbook xmlns=\"{NS}\" xmlns:r=\"{REL_NS}\"><bookViews><workbookView/></bookViews><sheets>");
    for (i, s) in sheets.iter().enumerate() {
        let _ = write!(x, "<sheet name=\"{}\" sheetId=\"{n}\" r:id=\"rId{n}\"/>", escape(s.name), n = i + 1);
    }
    x.push_str("</sheets></workbook>");
    x
}

fn workbook_rels(sheets: usize) -> String {
    let mut x = format!("{DECL}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">");
    for n in 1..=sheets {
        let _ = write!(x, "<Relationship Id=\"rId{n}\" Type=\"{REL_NS}/worksheet\" Target=\"worksheets/sheet{n}.xml\"/>");
    }
    let _ = write!(x, "<Relationship Id=\"rId{}\" Type=\"{REL_NS}/styles\" Target=\"styles.xml\"/></Relationships>", sheets + 1);
    x
}

/// One worksheet: an optional frozen header row, column widths, the cells.
fn sheet_xml(sheet: &Sheet) -> String {
    let mut x = format!("{DECL}<worksheet xmlns=\"{NS}\">");
    if sheet.freeze_header {
        x.push_str("<sheetViews><sheetView workbookViewId=\"0\"><pane ySplit=\"1\" topLeftCell=\"A2\" activePane=\"bottomLeft\" state=\"frozen\"/></sheetView></sheetViews>");
    }
    x.push_str("<cols>");
    for (i, w) in sheet.widths.iter().enumerate() {
        let _ = write!(x, "<col min=\"{n}\" max=\"{n}\" width=\"{w}\" customWidth=\"1\"/>", n = i + 1);
    }
    x.push_str("</cols><sheetData>");
    for (r, row) in sheet.rows.iter().enumerate() {
        let _ = write!(x, "<row r=\"{}\">", r + 1);
        for (c, cell) in row.iter().enumerate() {
            write_cell(&mut x, &format!("{}{}", column(c), r + 1), cell);
        }
        x.push_str("</row>");
    }
    x.push_str("</sheetData></worksheet>");
    x
}

fn write_cell(x: &mut String, at: &str, cell: &Cell) {
    let _ = match cell {
        Cell::Text(t, s) => write!(x, "<c r=\"{at}\" s=\"{s}\" t=\"inlineStr\"><is><t>{}</t></is></c>", escape(t)),
        Cell::Num(n, s) => write!(x, "<c r=\"{at}\" s=\"{s}\"><v>{n}</v></c>"),
        Cell::Date(d) => write!(x, "<c r=\"{at}\" s=\"{DATE}\"><v>{d:.6}</v></c>"),
        Cell::Empty => Ok(()),
    };
}

/// Column letters for a zero-based index: A..Z, AA..ZZ.
fn column(i: usize) -> String {
    let (q, r) = (i / 26, i % 26);
    let last = char::from(b'A' + r as u8);
    if q == 0 {
        last.to_string()
    } else {
        format!("{}{last}", char::from(b'A' + (q - 1) as u8))
    }
}

/// XML text: escapes the five specials and drops the control characters XML 1.0 forbids. Everything else,
/// emoji included, stays as UTF-8 (valid XML characters).
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' | '\n' | '\r' => out.push(ch),
            c if (c as u32) < 0x20 || c == '\u{FFFE}' || c == '\u{FFFF}' => {}
            c => out.push(c),
        }
    }
    out
}

const ROOT_RELS: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>";

/// Six cell formats, matching PLAIN..TOP above; numFmt 164 is the Riga date-time, 49 the built-in text format.
const STYLES: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><numFmts count=\"1\"><numFmt numFmtId=\"164\" formatCode=\"dd.mm.yyyy hh:mm\"/></numFmts><fonts count=\"2\"><font><sz val=\"11\"/><name val=\"Calibri\"/><family val=\"2\"/></font><font><b/><sz val=\"11\"/><color rgb=\"FFFFFFFF\"/><name val=\"Calibri\"/><family val=\"2\"/></font></fonts><fills count=\"3\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill><fill><patternFill patternType=\"solid\"><fgColor rgb=\"FF22313F\"/><bgColor indexed=\"64\"/></patternFill></fill></fills><borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs><cellXfs count=\"6\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/><xf numFmtId=\"0\" fontId=\"1\" fillId=\"2\" borderId=\"0\" xfId=\"0\" applyFont=\"1\" applyFill=\"1\" applyAlignment=\"1\"><alignment vertical=\"center\" wrapText=\"1\"/></xf><xf numFmtId=\"164\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyNumberFormat=\"1\" applyAlignment=\"1\"><alignment vertical=\"top\"/></xf><xf numFmtId=\"49\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyNumberFormat=\"1\" applyAlignment=\"1\"><alignment vertical=\"top\"/></xf><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyAlignment=\"1\"><alignment vertical=\"top\" wrapText=\"1\"/></xf><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyAlignment=\"1\"><alignment vertical=\"top\"/></xf></cellXfs><cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles></styleSheet>";
