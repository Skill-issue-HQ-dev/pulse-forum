//! Randomness from the operating system: the draw's random words and entry-shaped tokens.
//! Owns nothing else. Invariant: every value is read fresh from /dev/urandom (Unix), never seeded or reused.

use std::io::Read;

fn fill(buf: &mut [u8]) -> Result<(), String> {
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(buf))
        .map_err(|e| format!("random: /dev/urandom: {e}"))
}

/// One random 64-bit word, for the draw's rejection sampling.
pub fn next_u64() -> Result<u64, String> {
    let mut buf = [0u8; 8];
    fill(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

/// A token of the shape the phone makes: 16 random bytes as 32 lowercase hex characters.
pub fn token() -> Result<String, String> {
    let mut buf = [0u8; 16];
    fill(&mut buf)?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}
