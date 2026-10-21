//! A story entry as the page posts it, and the contract's rules applied to it in the contract's order.
//! Owns the input shape, trimming, the honeypot, the e-mail key and every field rule (docs/api.md).
//! Invariant: only an Entry that passed every rule reaches the store; the first failing rule names the error.

use crate::config;
use crate::core::labels;
use serde::Deserialize;

/// The body as the page sends it. Every field is optional here: a missing one (or null) counts as empty, so
/// the rules, not the parser, decide which code the page sees; a wrong type is a parse error (answered 400).
#[derive(Debug, Default, Deserialize)]
pub struct EntryInput {
    pub token: Option<String>,
    pub lang: Option<String>,
    pub category: Option<String>,
    pub story: Option<String>,
    pub first: Option<String>,
    pub last: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub company: Option<String>,
    pub consent: Option<bool>,
    pub share: Option<bool>,
    /// The honeypot: a hidden field only bots fill in.
    pub website: Option<String>,
}

/// A checked entry, trimmed, ready for the store. Consent is not kept: an entry only exists with it.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub token: String,
    pub lang: String,
    pub category: String,
    pub story: String,
    pub first: String,
    pub last: String,
    pub email: String,
    pub email_key: String,
    pub phone: String,
    pub company: String,
    pub share: bool,
}

/// One short-lived value per request, so the size gap between the variants costs nothing.
#[derive(Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Check {
    Valid(Entry),
    /// The honeypot was filled: answer as if stored, store nothing.
    Bot,
    /// The first failing rule's error code (token, lang, category, story_short, story_long, missing,
    /// too_long, email, phone, consent).
    Invalid(&'static str),
}

/// Trims every field, spots a bot, then applies the rules in the contract's order.
pub fn check(input: EntryInput) -> Check {
    if !text(input.website).is_empty() {
        return Check::Bot;
    }
    let email = text(input.email);
    let entry = Entry {
        token: text(input.token),
        lang: text(input.lang),
        category: text(input.category),
        story: text(input.story),
        first: text(input.first),
        last: text(input.last),
        email_key: email_key(&email),
        email,
        phone: text(input.phone),
        company: text(input.company),
        share: input.share.unwrap_or(false),
    };
    match rules(&entry, input.consent.unwrap_or(false)) {
        Ok(()) => Check::Valid(entry),
        Err(code) => Check::Invalid(code),
    }
}

fn text(v: Option<String>) -> String {
    v.map(|s| s.trim().to_string()).unwrap_or_default()
}

/// The ten rules, first failure wins: token, lang, category, story, missing, length, e-mail, phone, consent.
fn rules(e: &Entry, consent: bool) -> Result<(), &'static str> {
    if !token_ok(&e.token) {
        return Err("token");
    }
    if !labels::lang_ok(&e.lang) {
        return Err("lang");
    }
    if labels::category(&e.category).is_none() {
        return Err("category");
    }
    story_rule(&e.story)?;
    if [&e.first, &e.last, &e.email, &e.phone].iter().any(|f| f.is_empty()) {
        return Err("missing");
    }
    length_rule(e)?;
    if !email_ok(&e.email) {
        return Err("email");
    }
    if !phone_ok(&e.phone) {
        return Err("phone");
    }
    if !consent {
        return Err("consent");
    }
    Ok(())
}

fn story_rule(story: &str) -> Result<(), &'static str> {
    let n = story.chars().count();
    if n < config::STORY_MIN_CHARS {
        Err("story_short")
    } else if n > config::STORY_MAX_CHARS {
        Err("story_long")
    } else {
        Ok(())
    }
}

/// Lengths count characters, not bytes: one Latvian letter is one.
fn length_rule(e: &Entry) -> Result<(), &'static str> {
    let over = |s: &str, max: usize| s.chars().count() > max;
    let too_long = over(&e.first, config::NAME_MAX_CHARS)
        || over(&e.last, config::NAME_MAX_CHARS)
        || over(&e.email, config::EMAIL_MAX_CHARS)
        || over(&e.company, config::COMPANY_MAX_CHARS);
    if too_long {
        Err("too_long")
    } else {
        Ok(())
    }
}

/// The key that keeps one entry per person: the e-mail, trimmed and lower-cased.
pub fn email_key(email: &str) -> String {
    email.trim().to_lowercase()
}

/// One @, something before it, and a dot in the domain (not first) with two characters or more behind it.
pub fn email_ok(email: &str) -> bool {
    if email.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    let mut parts = email.split('@');
    let (Some(local), Some(domain), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let dot_with_tail = |(i, c): (usize, char)| c == '.' && i >= 1 && domain[i + 1..].chars().count() >= config::EMAIL_TLD_MIN_CHARS;
    !local.is_empty() && domain.char_indices().any(dot_with_tail)
}

/// A plus only first, then digits, spaces, hyphens, brackets and dots; 7 to 15 digits in all, and no more
/// than PHONE_MAX_CHARS characters (the contract caps digits only; this keeps a novel out of the column).
pub fn phone_ok(phone: &str) -> bool {
    if phone.chars().count() > config::PHONE_MAX_CHARS {
        return false;
    }
    let mut digits = 0;
    for (i, c) in phone.chars().enumerate() {
        match c {
            '0'..='9' => digits += 1,
            '+' if i == 0 => {}
            ' ' | '-' | '(' | ')' | '.' => {}
            _ => return false,
        }
    }
    (config::PHONE_MIN_DIGITS..=config::PHONE_MAX_DIGITS).contains(&digits)
}

/// An entry token: 32 lowercase hex characters (16 random bytes, made by the phone).
pub fn token_ok(token: &str) -> bool {
    token.len() == config::TOKEN_CHARS && token.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
