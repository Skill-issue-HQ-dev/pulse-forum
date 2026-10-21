//! The draw's pick: one index out of `len`, uniform, by rejection sampling over a source of random u64 words.
//! Owns the algorithm only, no randomness of its own (the io layer supplies the source).
//! Invariant: no modulo bias (words in the uneven top slice are thrown away), every index reachable, None only for len 0.

use std::convert::Infallible;

/// Picks an index in `0..len` from an infallible source of random words; None when `len` is 0.
pub fn pick(len: usize, next: &mut impl FnMut() -> u64) -> Option<usize> {
    match try_pick(len, &mut || Ok::<u64, Infallible>(next())) {
        Ok(picked) => picked,
        Err(never) => match never {},
    }
}

/// Same pick from a source that can fail (the operating system's randomness): the first error ends it.
pub fn try_pick<E>(len: usize, next: &mut impl FnMut() -> Result<u64, E>) -> Result<Option<usize>, E> {
    if len == 0 {
        return Ok(None);
    }
    let len = len as u64;
    // 2^64 mod len words at the top would make the low indexes likelier; they are rejected and redrawn.
    let uneven = (u64::MAX % len + 1) % len;
    loop {
        let word = next()?;
        if word <= u64::MAX - uneven {
            return Ok(Some((word % len) as usize));
        }
    }
}
