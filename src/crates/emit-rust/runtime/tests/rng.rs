//! The seeded generator behind property tests and laws: SplitMix64, the same
//! sequence on every machine.

/// SplitMix64.
#[derive(Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..bound`; `bound` is at least 1.
    pub fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound.max(1)
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }
}

/// Texts a generator draws from; a refined type keeps those its claim admits.
/// Edges on purpose: blanks, controls, Unicode white space and line breaks,
/// quotes and backslashes, non-ASCII, flag-like words.
pub const TEXTS: &[&str] = &[
    "milk",
    "  spaced  ",
    "caf\u{e9} \u{2615}",
    "say \"hi\"",
    "back\\slash",
    "\u{1F600}",
    "a\u{2028}b\u{85}c",
    "--all",
    "home",
    "work",
    "deep-work",
    "a",
    "42",
    "",
    "   ",
    "a\tb",
    "Home",
    "\u{3000}",
];
