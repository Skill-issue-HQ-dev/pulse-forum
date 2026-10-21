//! Cache stamps for the static site: every local file a page or script loads is referenced as `path?v=<hash>`, the
//! hash taken from that file's own stamped bytes, so a browser never mixes an old cached file with a new one.
//! Owns finding references [HTML src/href, CSS url(), JS import specifiers] and rewriting their stamps; no file I/O.
//! Invariant: stamping a stamped tree changes nothing [a fixed point], so a test can hold the committed site to it.

use std::collections::BTreeMap;

/// The site's files by path relative to its root ["img/sixt-pos.svg"], as bytes.
pub type Tree = BTreeMap<String, Vec<u8>>;

/// Files whose text can reference other files.
const TEXT: [&str; 3] = [".html", ".css", ".js"];
/// Where a reference starts; each ends at the closing quote [or bracket for an unquoted CSS url()].
const OPENERS: [&str; 5] = ["src=\"", "href=\"", "url(", "from \"", "import(\""];
/// Passes before giving up on a fixed point; the site's references form a shallow tree, so two or three suffice.
const MAX_PASSES: usize = 8;

/// A short content hash: 64-bit FNV-1a over the bytes, its first 8 hex digits.
pub fn hash(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")[..8].to_string()
}

/// Rewrites every local reference in every text file, pass after pass, until nothing changes.
pub fn stamp(tree: &Tree) -> Tree {
    let mut out = tree.clone();
    for _ in 0..MAX_PASSES {
        let hashes: BTreeMap<String, String> = out.iter().map(|(path, bytes)| (path.clone(), hash(bytes))).collect();
        let mut changed = false;
        for (path, bytes) in out.iter_mut() {
            if !TEXT.iter().any(|ext| path.ends_with(ext)) {
                continue;
            }
            let Ok(text) = std::str::from_utf8(bytes) else { continue };
            let new = restamp(path, text, &hashes);
            if new != text {
                *bytes = new.into_bytes();
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    out
}

/// One file's text with each reference to a file of the tree carrying that file's current hash.
fn restamp(path: &str, text: &str, hashes: &BTreeMap<String, String>) -> String {
    let dir = path.rfind('/').map_or("", |i| &path[..=i]);
    let mut out = String::with_capacity(text.len() + 64);
    let mut rest = text;
    while let Some((at, opener)) = next_opener(rest) {
        let value_start = at + opener.len();
        out.push_str(&rest[..value_start]);
        rest = &rest[value_start..];
        let quote = match rest.chars().next() {
            Some(q @ ('"' | '\'')) if opener == "url(" => Some(q),
            _ => None,
        };
        let skip = quote.map_or(0, char::len_utf8);
        let close = if opener == "url(" { quote.unwrap_or(')') } else { '"' };
        let Some(len) = rest[skip..].find(close) else { break };
        let value = &rest[skip..skip + len];
        out.push_str(&rest[..skip]);
        out.push_str(&stamped(dir, value, hashes));
        rest = &rest[skip + len..];
    }
    out.push_str(rest);
    out
}

/// The earliest reference opener in `text` and its position.
fn next_opener(text: &str) -> Option<(usize, &'static str)> {
    OPENERS.iter().filter_map(|o| text.find(o).map(|i| (i, *o))).min_by_key(|(i, _)| *i)
}

/// `value` with its stamp renewed when it names a file of the tree, else unchanged.
fn stamped(dir: &str, value: &str, hashes: &BTreeMap<String, String>) -> String {
    let bare = value.split_once("?v=").map_or(value, |(path, _)| path);
    let target = format!("{dir}{}", bare.trim_start_matches("./"));
    match hashes.get(&target) {
        Some(h) => format!("{bare}?v={h}"),
        None => value.to_string(),
    }
}
