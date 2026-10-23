//! The fixed vocabularies of an entry: the eight story categories with their Latvian and English labels,
//! and the two page languages. Owns the codes as the page sends them and the labels the export shows.
//! Invariant: a code is valid exactly when it is listed here.

/// One story category: the code the page posts and its two labels.
pub struct Category {
    pub code: &'static str,
    pub lv: &'static str,
    pub en: &'static str,
}

/// In the page's order [frontend/strings.js]: two per row on a computer, as the organisers laid them out.
pub const CATEGORIES: [Category; 8] = [
    Category { code: "collaboration", lv: "Sadarbība", en: "Collaboration" },
    Category { code: "business", lv: "Uzņēmuma attīstība", en: "Company growth" },
    Category { code: "idea", lv: "Jauna biznesa ideja", en: "A new business idea" },
    Category { code: "event", lv: "Pasākuma apmeklējums", en: "An event" },
    Category { code: "career", lv: "Karjeras pārmaiņas", en: "A career change" },
    Category { code: "friendship", lv: "Jauna draudzība", en: "A new friendship" },
    Category { code: "trip", lv: "Ceļojums", en: "A trip" },
    Category { code: "other", lv: "Cits, vēl labāks stāsts!", en: "Another, even better story!" },
];

/// The page's languages.
pub const LANGS: [&str; 2] = ["lv", "en"];

/// The category behind a code, if it is one of the eight.
pub fn category(code: &str) -> Option<&'static Category> {
    CATEGORIES.iter().find(|c| c.code == code)
}

pub fn lang_ok(code: &str) -> bool {
    LANGS.contains(&code)
}
