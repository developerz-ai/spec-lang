//! The engine's generator, draw for draw: SplitMix64, seeds from FNV-1a of a check's name, and the base values of
//! `Int` and `Text`. The same seed gives the engine's inputs.

use super::Int;

/// Tries per value before a claim counts as unmet.
pub const TRIES: u32 = 400;

#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `0..n` (`0` when `n == 0`).
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }

    /// In `lo..=hi`: two draws make a 128-bit number, high first, modulo the span.
    pub fn between(&mut self, lo: Int, hi: Int) -> Int {
        if hi <= lo {
            return lo;
        }
        let span = hi.abs_diff(lo) + 1;
        let r = ((u128::from(self.next_u64()) << 64) | u128::from(self.next_u64())) % span;
        lo.wrapping_add_unsigned(r)
    }

    /// True with probability `num / den`.
    pub fn chance(&mut self, num: u64, den: u64) -> bool {
        self.below(den) < num
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> Option<&'a T> {
        xs.get(usize::try_from(self.below(xs.len() as u64)).unwrap_or(0))
    }

    /// A count `0..n` as a loop bound.
    pub fn count(&mut self, n: u64) -> usize {
        usize::try_from(self.below(n)).unwrap_or(0)
    }
}

/// The seed of run `index` of the check `name`.
pub fn seed_for(base: u64, name: &str, index: u64) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in name.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    Rng::new(h ^ base.rotate_left(17) ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15)).next_u64()
}

/// Small numbers most of the time, then boundaries a machine word breaks at.
pub fn int(rng: &mut Rng) -> Int {
    const EDGES: &[Int] = &[
        0,
        1,
        -1,
        2_147_483_647,
        2_147_483_648,
        9_007_199_254_740_992,
        9_007_199_254_740_993,
        9_223_372_036_854_775_807,
        9_223_372_036_854_775_808,
        -9_223_372_036_854_775_808,
        18_446_744_073_709_551_616,
    ];
    match rng.below(100) {
        0..35 => rng.between(0, 10),
        35..55 => rng.between(11, 100),
        55..70 => rng.between(-10, -1),
        70..80 => rng.pick(EDGES).copied().unwrap_or(0),
        80..90 => rng.between(101, 1 << 62),
        _ => -rng.between(11, 1 << 62),
    }
}

/// Text from one of four alphabets: shell-safe words, printable ASCII, a mix with Unicode space, controls and
/// astral characters, or empty.
pub fn text(rng: &mut Rng) -> String {
    const WORD: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789-";
    const MIX: &[char] = &[
        'a', 'Z', ' ', '\t', '\n', 'é', 'ß', '中', '😀', '\u{3000}', '\u{85}', '\u{A0}', '\u{200B}', '\u{FEFF}',
        '\u{7F}', '\u{1}', '"', '\\', '{', '}', '#', '-',
    ];
    match rng.below(4) {
        0 => {
            let n = 1 + rng.below(8);
            (0..n).map(|_| char::from(*rng.pick(WORD).unwrap_or(&b'a'))).collect()
        }
        1 => {
            let n = rng.below(13);
            (0..n).map(|_| char::from(u8::try_from(32 + rng.below(95)).unwrap_or(b'a'))).collect()
        }
        2 => {
            let n = rng.below(9);
            (0..n).map(|_| *rng.pick(MIX).unwrap_or(&'a')).collect()
        }
        _ => String::new(),
    }
}

/// A value of a range type: one of its ends one time in five, else uniform.
pub fn in_range(rng: &mut Rng, lo: Int, hi: Int) -> Int {
    if rng.chance(1, 5) {
        if rng.chance(1, 2) { lo } else { hi }
    } else {
        rng.between(lo, hi)
    }
}

/// Half the time, when the pool holds values, one of them: the draws the engine makes for a pooled type.
pub fn from_pool<T: Clone>(rng: &mut Rng, pool: &[T]) -> Option<T> {
    if pool.is_empty() || !rng.chance(1, 2) {
        return None;
    }
    rng.pick(pool).cloned()
}
