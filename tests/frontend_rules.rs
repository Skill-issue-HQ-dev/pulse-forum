//! The page asks for what the collector takes: the eight categories in the same order with the same labels, and
//! the same field limits. Owns the check between frontend/strings.js + frontend/form.js and src/.
//! Invariant: a category or a limit changed on one side only fails here, before a guest meets a refused entry.

use pulse_forum::config;
use pulse_forum::core::labels::CATEGORIES;
use std::fs;

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

#[test]
fn the_page_offers_the_collectors_categories_in_its_order() {
    let strings = read("frontend/strings.js");
    let codes: Vec<String> = CATEGORIES.iter().map(|c| format!("\"{}\"", c.code)).collect();
    let line = format!("export const CATEGORIES = [{}];", codes.join(", "));
    assert!(strings.contains(&line), "strings.js must carry {line}");
    for c in &CATEGORIES {
        for label in [c.lv, c.en] {
            assert!(strings.contains(&format!("{}: \"{label}\",", c.code)), "strings.js must label {} {label:?}", c.code);
        }
    }
}

#[test]
fn the_page_checks_the_collectors_limits() {
    let form = read("frontend/form.js");
    let limits = format!(
        "storyMin: 1, storyMax: {}, name: {}, email: {}, company: {}, phoneDigitsMin: {}, phoneDigitsMax: {} }}",
        config::STORY_MAX_CHARS,
        config::NAME_MAX_CHARS,
        config::EMAIL_MAX_CHARS,
        config::COMPANY_MAX_CHARS,
        config::PHONE_MIN_DIGITS,
        config::PHONE_MAX_DIGITS
    );
    assert!(form.contains(&limits), "form.js LIMITS must read {limits}");
    let counter = format!("count: (n) => `${{n}} / {}`", config::STORY_MAX_CHARS);
    assert_eq!(read("frontend/strings.js").matches(&counter).count(), 2, "both languages count to the story limit");
}
