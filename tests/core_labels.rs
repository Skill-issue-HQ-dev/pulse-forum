//! Tests for core::labels: the eight categories with the contract's Latvian and English labels in the page's
//! order, only listed codes count as categories [a retired one too], and the page speaks Latvian or English.

use pulse_forum::core::labels::{category, lang_ok, CATEGORIES};

#[test]
fn the_eight_categories_carry_the_contracts_labels() {
    let table = [
        ("collaboration", "Sadarbība", "Collaboration"),
        ("business", "Uzņēmuma attīstība", "Company growth"),
        ("idea", "Jauna biznesa ideja", "A new business idea"),
        ("event", "Pasākuma apmeklējums", "An event"),
        ("career", "Karjeras pārmaiņas", "A career change"),
        ("friendship", "Jauna draudzība", "A new friendship"),
        ("trip", "Ceļojums", "A trip"),
        ("other", "Cits, vēl labāks stāsts!", "Another, even better story!"),
    ];
    assert_eq!(CATEGORIES.len(), table.len());
    let order: Vec<&str> = CATEGORIES.iter().map(|c| c.code).collect();
    assert_eq!(order, table.map(|t| t.0), "the page's order: two per row on a computer");
    for (code, lv, en) in table {
        let c = category(code).unwrap_or_else(|| panic!("{code} is not listed"));
        assert_eq!((c.code, c.lv, c.en), (code, lv, en));
    }
}

#[test]
fn only_listed_codes_are_categories() {
    for bad in ["", "new_contact", "FRIENDSHIP", "new friendship", "Jauna draudzība", "nope"] {
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
