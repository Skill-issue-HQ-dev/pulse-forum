//! Tests for core::draw: a uniform pick by rejection sampling, driven by deterministic word sources: every index
//! reachable, the rejection path exercised, the accept boundary exact for several sizes, no modulo bias.

use pulse_forum::core::draw::{pick, try_pick};

/// A source that hands out these words in order and counts the calls.
struct Words {
    words: Vec<u64>,
    calls: usize,
}

impl Words {
    fn new(words: &[u64]) -> Words {
        Words { words: words.to_vec(), calls: 0 }
    }
    fn next(&mut self) -> u64 {
        let w = self.words[self.calls];
        self.calls += 1;
        w
    }
}

/// splitmix64: a small, fixed, well-mixed generator for the statistical test (the same numbers on every run).
fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[test]
fn an_empty_pool_has_no_pick_and_asks_for_no_randomness() {
    let mut w = Words::new(&[]);
    assert_eq!(pick(0, &mut || w.next()), None);
    assert_eq!(w.calls, 0);
}

#[test]
fn a_pool_of_one_always_picks_it() {
    for word in [0, 1, u64::MAX / 2, u64::MAX] {
        let mut w = Words::new(&[word]);
        assert_eq!(pick(1, &mut || w.next()), Some(0));
        assert_eq!(w.calls, 1, "a power of two rejects nothing");
    }
}

#[test]
fn every_index_is_reachable() {
    for len in [2usize, 3, 7, 10, 64, 1000] {
        let mut seen = vec![false; len];
        for word in 0..len as u64 {
            let mut w = Words::new(&[word]);
            let i = pick(len, &mut || w.next()).unwrap();
            assert_eq!(i, word as usize, "word {word} of {len}");
            seen[i] = true;
        }
        assert!(seen.iter().all(|s| *s), "len {len}");
    }
}

#[test]
fn a_word_in_the_uneven_top_slice_is_thrown_away_and_redrawn() {
    // 2^64 mod 10 = 6: the six highest words would make indexes 0..=5 likelier, so they are rejected.
    let mut w = Words::new(&[u64::MAX, u64::MAX - 5, u64::MAX - 6]);
    assert_eq!(pick(10, &mut || w.next()), Some(9), "(2^64 - 7) mod 10");
    assert_eq!(w.calls, 3, "two rejections, then the third word counts");
}

#[test]
fn the_accept_boundary_is_exact_for_many_sizes() {
    for len in [1usize, 2, 3, 5, 6, 7, 10, 100, 1000, 65_537, 1_000_003, 4_000_000_007] {
        let uneven = ((1u128 << 64) % len as u128) as u64;
        let last_accepted = u64::MAX - uneven;
        let mut w = Words::new(&[last_accepted]);
        let got = pick(len, &mut || w.next()).unwrap();
        assert_eq!((got, w.calls), ((last_accepted % len as u64) as usize, 1), "len {len}: last accepted word");
        if uneven > 0 {
            let mut w = Words::new(&[last_accepted + 1, 0]);
            assert_eq!(pick(len, &mut || w.next()), Some(0), "len {len}: first rejected word");
            assert_eq!(w.calls, 2, "len {len}: the first rejected word is drawn again");
        }
        // The accepted words are a whole number of laps over the indexes, which is what removes the bias.
        assert_eq!((u64::MAX as u128 + 1 - uneven as u128) % len as u128, 0, "len {len}");
    }
}

#[test]
fn a_sweep_of_consecutive_words_hits_every_index_equally() {
    let len = 6usize;
    let mut counts = vec![0usize; len];
    for word in 0..6000u64 {
        let mut w = Words::new(&[word]);
        counts[pick(len, &mut || w.next()).unwrap()] += 1;
    }
    assert!(counts.iter().all(|c| *c == 1000), "{counts:?}");
}

#[test]
fn a_fixed_pseudo_random_stream_comes_out_flat() {
    let (len, draws) = (7usize, 70_000usize);
    let mut state = 0x5EED_u64;
    let mut counts = vec![0usize; len];
    let mut next = || splitmix(&mut state);
    for _ in 0..draws {
        counts[pick(len, &mut next).unwrap()] += 1;
    }
    let expected = draws / len;
    for (i, c) in counts.iter().enumerate() {
        assert!(c.abs_diff(expected) < expected / 20, "index {i}: {c} vs {expected} ({counts:?})");
    }
}

#[test]
fn a_failing_source_ends_the_pick_with_its_error() {
    let mut calls = 0;
    let mut next = || {
        calls += 1;
        Err::<u64, &str>("urandom is gone")
    };
    assert_eq!(try_pick(5, &mut next), Err("urandom is gone"));
    assert_eq!(calls, 1);
    let mut never = || -> Result<u64, &str> { panic!("an empty pool must not read randomness") };
    assert_eq!(try_pick(0, &mut never), Ok(None));
}
