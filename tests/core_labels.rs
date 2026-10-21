//! Tests for core::labels: the eight categories with the contract's Latvian and English labels,
//! only listed codes count as categories, and the page speaks Latvian or English.

use pulse_forum::core::labels::{category, lang_ok, CATEGORIES};

#[test]
fn the_eight_categories_carry_the_contracts_labels() {
    let table = [
        ("new_contact", "Jauns kontakts", "A new contact"),
        ("business", "Biznesa attīstība", "Business growth"),
        ("collaboration", "Sadarbība", "Collaboration"),
        ("event", "Pasākuma apmeklējums", "An event"),
        ("idea", "Jauna biznesa ideja", "A new business idea"),
        ("career", "Karjeras pavērsiens", "A career move"),
        ("trip", "Ceļojums", "A trip"),
        ("other", "Cits", "Something else"),
    ];
    assert_eq!(CATEGORIES.len(), table.len());
    for (code, lv, en) in table {
        let c = category(code).unwrap_or_else(|| panic!("{code} is not listed"));
        assert_eq!((c.code, c.lv, c.en), (code, lv, en));
    }
}

#[test]
fn only_listed_codes_are_categories() {
    for bad in ["", "NEW_CONTACT", "new contact", "Jauns kontakts", "nope"] {
        assert!(category(bad).is_none(), "{bad}");
    }
}

#[test]
fn the_page_speaks_latvian_or_english() {
    assert!(lang_ok("lv") && lang_ok("en"));
    for bad in ["", "LV", "ru", "lv ", "lav"] {
        assert!(!lang_ok(bad), "{bad:?}");
    }
}
