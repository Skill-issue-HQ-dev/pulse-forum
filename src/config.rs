//! Every constant of the service in one place: ports, environment names, field limits, headers, file names.
//! Owns the numbers and words the contract (docs/api.md) names; no other file holds a magic value.
//! Invariant: a limit changes here only, and the page's own rules must follow the same numbers.

/// Loopback port of `pulse-forum serve` (env PULSE_PORT overrides).
pub const DEFAULT_PORT: u16 = 8820;
pub const PORT_ENV: &str = "PULSE_PORT";
/// Folder of the entries database (env PULSE_DATA; the server's unit points it at its own state directory).
pub const DATA_ENV: &str = "PULSE_DATA";
pub const DEFAULT_DATA_DIR: &str = "data";
pub const DB_FILE: &str = "entries.sqlite3";
/// The admin key comes from this variable and must hold at least MIN_KEY_CHARS characters.
pub const ADMIN_KEY_ENV: &str = "PULSE_ADMIN_KEY";
pub const MIN_KEY_CHARS: usize = 32;
/// Browser origins allowed by CORS, comma separated (env PULSE_ORIGINS), and the default list.
pub const ORIGINS_ENV: &str = "PULSE_ORIGINS";
pub const DEFAULT_ORIGINS: &str = "https://skill-issue-hq-dev.github.io";

/// Request body cap in bytes; a larger body is answered 413.
pub const MAX_BODY_BYTES: usize = 16 * 1024;

/// A phone-made entry token: 32 lowercase hex characters.
pub const TOKEN_CHARS: usize = 32;
/// Field limits in characters (not bytes), counted after trimming.
pub const STORY_MIN_CHARS: usize = 20;
pub const STORY_MAX_CHARS: usize = 1500;
pub const NAME_MAX_CHARS: usize = 80;
pub const EMAIL_MAX_CHARS: usize = 120;
pub const COMPANY_MAX_CHARS: usize = 120;
/// One person, one chance: entries whose phones end in the same this-many digits count as one person.
pub const PHONE_KEY_DIGITS: usize = 8;
/// The contract gives the phone a digit range only; this cap on its characters (a phone error) keeps a novel out.
pub const PHONE_MAX_CHARS: usize = 40;
pub const PHONE_MIN_DIGITS: usize = 7;
pub const PHONE_MAX_DIGITS: usize = 15;
/// An e-mail needs a dot with at least this many characters after it.
pub const EMAIL_TLD_MIN_CHARS: usize = 2;

/// Wall clock of the workbook and of its file name.
pub const TIME_ZONE: &str = "Europe/Riga";
/// The word `clear` must be sent with before it deletes anything.
pub const CLEAR_WORD: &str = "DZĒST";

/// Response headers every answer carries.
pub const SECURITY_HEADERS: &[(&str, &str)] = &[
    ("content-security-policy", "default-src 'none'; frame-ancestors 'none'"),
    ("x-content-type-options", "nosniff"),
    ("referrer-policy", "no-referrer"),
];
/// CORS answers: the preflight's allow list and what the page may read back from a download.
pub const CORS_METHODS: &str = "GET, POST, OPTIONS";
pub const CORS_HEADERS: &str = "authorization, content-type";
pub const CORS_MAX_AGE: &str = "600";
pub const CORS_EXPOSE: &str = "content-disposition";

/// The workbook: media type and the file-name stem (the Riga date and time follow it).
pub const XLSX_TYPE: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
pub const EXPORT_STEM: &str = "PULSE izloze";
