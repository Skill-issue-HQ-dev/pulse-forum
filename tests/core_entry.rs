//! Tests for core::entry: every 422 code and their order, trimming, characters (not bytes), the entry without a
//! story [the break option], the honeypot, the e-mail key, and the e-mail, phone and token rules.

use pulse_forum::core::entry::{Check, EntryInput, check, email_key, email_ok, phone_key, phone_ok, token_ok};
use serde_json::{json, Value};

const TOKEN: &str = "0123456789abcdef0123456789abcdef";

fn base() -> Value {
    json!({
        "token": TOKEN, "lang": "lv", "category": "trip",
        "story": "Satiku jaunu cilvēku pasākumā un mēs sākām sarunu.",
        "first": "Testa", "last": "Ķirsis", "email": "Test@Example.com", "phone": "+371 20 000 000",
        "company": "Parauga SIA", "consent": true, "share": true, "website": ""
    })
}

fn with(field: &str, value: Value) -> Value {
    let mut v = base();
    v[field] = value;
    v
}

fn input(v: &Value) -> EntryInput {
    serde_json::from_value(v.clone()).unwrap()
}

fn code(v: &Value) -> &'static str {
    match check(input(v)) {
        Check::Invalid(c) => c,
        other => panic!("expected a refusal, got {other:?} for {v}"),
    }
}

fn is_valid(v: &Value) -> bool {
    matches!(check(input(v)), Check::Valid(_))
}

#[test]
fn a_complete_entry_is_valid_and_stored_trimmed() {
    let mut v = base();
    for f in ["lang", "category", "story", "first", "last", "email", "phone", "company"] {
        v[f] = json!(format!("  {} \n", v[f].as_str().unwrap()));
    }
    let Check::Valid(e) = check(input(&v)) else { panic!("not valid") };
    assert_eq!((e.token.as_str(), e.lang.as_str(), e.category.as_str()), (TOKEN, "lv", "trip"));
    assert_eq!((e.first.as_str(), e.last.as_str(), e.company.as_str()), ("Testa", "Ķirsis", "Parauga SIA"));
    assert_eq!((e.email.as_str(), e.email_key.as_str()), ("Test@Example.com", "test@example.com"));
    assert_eq!((e.phone.as_str(), e.story.as_str()), ("+371 20 000 000", "Satiku jaunu cilvēku pasākumā un mēs sākām sarunu."));
    assert!(e.share);
}

#[test]
fn missing_and_null_fields_count_as_empty() {
    // Nothing at all: the first rule (token) speaks, not the parser.
    assert_eq!(code(&json!({})), "token");
    // company, share and website are optional; null is the same as absent.
    let mut v = base();
    v["company"] = Value::Null;
    v["share"] = Value::Null;
    v["website"] = Value::Null;
    let Check::Valid(e) = check(input(&v)) else { panic!("not valid") };
    assert_eq!((e.company.as_str(), e.share), ("", false));
}

#[test]
fn wrong_field_types_are_a_parse_error_not_a_rule() {
    for (field, value) in [("consent", json!("yes")), ("share", json!(1)), ("story", json!(5)), ("token", json!(["a"])), ("website", json!(true))] {
        assert!(serde_json::from_value::<EntryInput>(with(field, value)).is_err(), "{field}");
    }
}

#[test]
fn every_error_code_has_a_trigger() {
    let long = |n: usize| "x".repeat(n);
    let cases: Vec<(&str, Value)> = vec![
        ("token", with("token", json!(TOKEN.to_uppercase()))),
        ("token", with("token", json!("abc"))),
        ("token", with("token", json!(""))),
        ("lang", with("lang", json!("de"))),
        ("lang", with("lang", json!("LV"))),
        ("lang", with("lang", json!(""))),
        ("category", with("category", json!("nope"))),
        ("category", with("category", json!("New_Contact"))),
        ("story_long", with("story", json!(long(501)))),
        ("missing", with("first", json!("  "))),
        ("missing", with("last", json!(""))),
        ("missing", with("email", json!(""))),
        ("missing", with("phone", json!(" "))),
        ("too_long", with("first", json!(long(81)))),
        ("too_long", with("last", json!(long(81)))),
        ("too_long", with("email", json!(format!("{}@b.co", long(116))))),
        ("too_long", with("company", json!(long(121)))),
        ("email", with("email", json!("nobody"))),
        ("email", with("email", json!("a@b"))),
        ("email", with("email", json!("a@b.c"))),
        ("phone", with("phone", json!("123456"))),
        ("phone", with("phone", json!("1234567890123456"))),
        ("phone", with("phone", json!("2000+0000"))),
        ("phone", with("phone", json!("call me maybe"))),
        ("phone", with("phone", json!(format!("1234567{}", "-".repeat(40))))),
        ("consent", with("consent", json!(false))),
    ];
    for (want, v) in &cases {
        assert_eq!(code(v), *want, "{v}");
    }
    // A missing consent is a refused consent.
    let mut v = base();
    v.as_object_mut().unwrap().remove("consent");
    assert_eq!(code(&v), "consent");
    let mut seen: Vec<&str> = cases.iter().map(|(c, _)| *c).chain(["consent"]).collect();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), 9, "all nine codes are covered: {seen:?}");
}

#[test]
fn the_first_failing_rule_wins_in_the_contracts_order() {
    // Everything is wrong at once; each step fixes only the rule that just spoke, so the next one must speak.
    let mut v = json!({
        "token": "zz", "lang": "de", "category": "nope", "story": "x".repeat(501),
        "first": "", "last": "Ķirsis", "email": "e".repeat(121), "phone": "x", "company": "", "consent": false
    });
    let steps: [(&str, &str, Value); 9] = [
        ("token", "token", json!(TOKEN)),
        ("lang", "lang", json!("en")),
        ("category", "category", json!("idea")),
        ("story_long", "story", base()["story"].clone()),
        ("missing", "first", json!("Testa")),
        ("too_long", "email", json!("e@x")),
        ("email", "email", json!("a@b.co")),
        ("phone", "phone", json!("+371 20 000 000")),
        ("consent", "consent", json!(true)),
    ];
    for (expected, field, fix) in steps {
        assert_eq!(code(&v), expected);
        v[field] = fix;
    }
    assert!(is_valid(&v), "after the last fix nothing is left");
}

#[test]
fn lengths_count_characters_not_bytes() {
    // Latvian letters are two bytes each, an emoji four: all count once.
    assert!(is_valid(&with("story", json!("ā"))));
    assert!(is_valid(&with("story", json!("ū".repeat(500)))));
    assert_eq!(code(&with("story", json!("ū".repeat(501)))), "story_long");
    assert!(is_valid(&with("story", json!("😀".repeat(500)))));
    assert_eq!(code(&with("story", json!("😀".repeat(501)))), "story_long");
    assert!(is_valid(&with("first", json!("ž".repeat(80)))));
    assert_eq!(code(&with("first", json!("ž".repeat(81)))), "too_long");
    assert!(is_valid(&with("last", json!("Ķ".repeat(80)))));
    assert!(is_valid(&with("company", json!("ķ".repeat(120)))));
    assert_eq!(code(&with("company", json!("ķ".repeat(121)))), "too_long");
    assert!(is_valid(&with("email", json!(format!("{}@b.co", "ā".repeat(115))))));
    assert_eq!(code(&with("email", json!(format!("{}@b.co", "ā".repeat(116))))), "too_long");
}

#[test]
fn length_rules_run_after_trimming() {
    let padded_ok = format!("  {}  ", "x".repeat(500));
    assert!(is_valid(&with("story", json!(padded_ok))));
    let padded_long = format!(" {} ", "x".repeat(501));
    assert_eq!(code(&with("story", json!(padded_long))), "story_long");
    assert!(is_valid(&with("first", json!(format!(" {} ", "a".repeat(80))))));
}

#[test]
fn an_empty_story_is_the_break_option_and_still_enters() {
    // The guest ticked "Dodos pauzē…": the page sends no story; spaces alone or no field at all count the same.
    let mut missing = base();
    missing.as_object_mut().unwrap().remove("story");
    for v in [with("story", json!("")), with("story", json!("  \n ")), missing] {
        match check(input(&v)) {
            Check::Valid(e) => assert_eq!(e.story, "", "{v}"),
            other => panic!("expected a valid entry, got {other:?} for {v}"),
        }
    }
}

#[test]
fn the_honeypot_beats_every_rule_and_whitespace_is_not_a_bot() {
    assert_eq!(check(input(&with("website", json!("http://spam.example")))), Check::Bot);
    let mut broken = json!({ "website": "x" });
    assert_eq!(check(input(&broken)), Check::Bot, "even an otherwise empty body");
    broken["website"] = json!("   ");
    assert_eq!(code(&broken), "token", "blanks are not a filled honeypot");
    assert!(is_valid(&with("website", json!("  "))));
}

#[test]
fn the_email_key_folds_case_and_trims() {
    assert_eq!(email_key(" Anna@Example.COM "), "anna@example.com");
    assert_eq!(email_key("ĀBOLS@Example.Com"), "ābols@example.com");
    let Check::Valid(a) = check(input(&with("email", json!(" Test@EXAMPLE.com ")))) else { panic!() };
    let Check::Valid(b) = check(input(&with("email", json!("test@example.COM")))) else { panic!() };
    assert_eq!(a.email_key, b.email_key);
    assert_ne!(a.email, b.email, "the address itself is kept as typed");
}

#[test]
fn the_email_rule() {
    for ok in ["a@b.co", "first.last@sub.example.lv", "x@b.cd.e", "ā@b.lv", "a+tag@example.com"] {
        assert!(email_ok(ok), "{ok}");
    }
    for bad in ["a@b", "a@.co", "@b.co", "a b@c.co", "a@b@c.co", "a@b.c", "", "plain", "a@b.co\n", "a\u{0}@b.co", "a@b.ā"] {
        assert!(!email_ok(bad), "{bad:?}");
    }
}

#[test]
fn the_phone_rule() {
    for ok in ["+371 20 000 000", "20000000", "20-000-000", "+371 (20) 000.000", "1234567", "+123456789012345", "+ 371 20 000 000"] {
        assert!(phone_ok(ok), "{ok}");
    }
    for bad in ["", "+371", "123456", "1234567890123456", "371+20000000", "++37120000000", "20000000x", "ab1234567", "+-()", "2000 0000\n", "٢٠٠٠٠٠٠٠", "+371 20 000 000 ext 5"] {
        assert!(!phone_ok(bad), "{bad:?}");
    }
}

#[test]
fn a_phone_longer_than_40_characters_is_a_phone_error_not_too_long() {
    let padded = |spaces: usize| format!("+371 20 000 000{}", " ".repeat(spaces).replace(' ', "-"));
    assert_eq!(padded(0).chars().count(), 15);
    assert!(phone_ok(&padded(25)), "exactly 40 characters");
    assert!(!phone_ok(&padded(26)), "41 characters");
    assert_eq!(code(&with("phone", json!(padded(26)))), "phone");
}

#[test]
fn the_token_rule() {
    assert!(token_ok(TOKEN));
    assert!(token_ok(&"0".repeat(32)));
    for bad in [TOKEN.to_uppercase(), TOKEN[1..].to_string(), format!("{TOKEN}0"), "g".repeat(32), String::new(), format!("é{}", &TOKEN[2..])] {
        assert!(!token_ok(&bad), "{bad:?}");
    }
}

#[test]
fn the_phone_key_is_the_last_eight_digits() {
    assert_eq!(phone_key("+371 20 000 001"), "20000001");
    assert_eq!(phone_key("2000-0001"), "20000001", "same number without the country code");
    assert_eq!(phone_key("+44 20 7946 0958"), "79460958");
    assert_eq!(phone_key("123 4567"), "1234567", "a short number keeps all its digits");
}
