//! Tests for core::auth: the bearer header parsing and the constant-time comparison.
//! Functional only: a test cannot time the comparison, so it checks length and every byte instead.

use pulse_forum::core::auth::{bearer_ok, same};

const KEY: &str = "0123456789abcdef0123456789abcdef-key";

#[test]
fn the_exact_key_as_a_bearer_token_passes() {
    assert!(bearer_ok(format!("Bearer {KEY}").as_bytes(), KEY));
    assert!(bearer_ok(format!("bearer {KEY}").as_bytes(), KEY), "the scheme is case-insensitive");
    assert!(bearer_ok(format!("BEARER  {KEY} ").as_bytes(), KEY), "spaces around the token do not count");
}

#[test]
fn anything_else_is_refused() {
    let shorter = &KEY[..KEY.len() - 1];
    let mut flipped = KEY.to_string();
    flipped.replace_range(0..1, "1");
    let mut last_flipped = KEY.to_string();
    last_flipped.replace_range(KEY.len() - 1.., "Y");
    let refused = [
        String::new(),
        "Bearer".to_string(),
        "Bearer ".to_string(),
        KEY.to_string(),
        format!("Basic {KEY}"),
        format!("Token {KEY}"),
        format!("Bearer {shorter}"),
        format!("Bearer {KEY}x"),
        format!("Bearer {KEY}\0"),
        format!("Bearer {flipped}"),
        format!("Bearer {last_flipped}"),
        format!("Bearer {KEY} {KEY}"),
        format!("Bearer{KEY}"),
    ];
    for header in &refused {
        assert!(!bearer_ok(header.as_bytes(), KEY), "{header:?}");
    }
}

#[test]
fn every_prefix_of_the_key_is_refused() {
    for n in 0..KEY.len() {
        assert!(!bearer_ok(format!("Bearer {}", &KEY[..n]).as_bytes(), KEY), "prefix of {n}");
    }
}

#[test]
fn the_comparison_sees_length_and_every_byte() {
    assert!(same(b"abc", b"abc"));
    assert!(!same(b"abc", b"abd"));
    assert!(!same(b"abc", b"ab"));
    assert!(!same(b"ab", b"abc"));
    assert!(!same(b"abc\0", b"abc"), "trailing zero bytes are not padding");
    assert!(!same(b"", b"abc"));
    assert!(same(b"", b""));
    assert!(!same(&[0xFF; 4], &[0x7F; 4]));
}

#[test]
fn non_ascii_bytes_cannot_panic_or_match() {
    assert!(!bearer_ok(&[0xFF, 0xFE, 0x20, 0x80], KEY));
    assert!(!bearer_ok(b"Bearer \xFF\xFE", KEY));
    assert!(bearer_ok("Bearer ā-key".as_bytes(), "ā-key"));
}
