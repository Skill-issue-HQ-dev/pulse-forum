//! The admin key check: `Authorization: Bearer <key>` compared in constant time, no crate needed.
//! Owns the header parsing and the comparison; the key is only passed in, never stored or logged here.
//! Invariant: the comparison visits every byte of the real key whatever the caller sent, so timing says nothing about a guess.

use std::hint::black_box;

/// True when the header value is `Bearer <key>` (scheme in any case) and the token equals `key`.
pub fn bearer_ok(header: &[u8], key: &str) -> bool {
    match bearer_token(header) {
        Some(token) => same(token, key.as_bytes()),
        None => false,
    }
}

fn bearer_token(header: &[u8]) -> Option<&[u8]> {
    let space = header.iter().position(|b| *b == b' ')?;
    let (scheme, rest) = header.split_at(space);
    scheme.eq_ignore_ascii_case(b"bearer").then(|| rest.trim_ascii())
}

/// Equal bytes, in time that depends only on the length of `expected`: a different length or any
/// different byte sets a bit in `diff`, and the loop never stops early.
pub fn same(presented: &[u8], expected: &[u8]) -> bool {
    let mut diff = presented.len() ^ expected.len();
    for (i, want) in expected.iter().enumerate() {
        let got = presented.get(i).copied().unwrap_or(0);
        diff |= black_box(usize::from(got ^ want));
    }
    black_box(diff) == 0
}
