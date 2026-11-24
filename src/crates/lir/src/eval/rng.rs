//! The generator every printer's test runtime implements, so a printed
//! property or law draws the engine's inputs: SplitMix64, and seeds from
//! FNV-1a of the check's name (`README.md`, Seeds).

/// SplitMix64.
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

    /// Uniform in `0..n` (`0` when `n == 0`): `next_u64() % n`.
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next_u64() % n }
    }

    /// In `lo..=hi`: two draws make a 128-bit number, high first, modulo the span.
    pub fn between(&mut self, lo: i128, hi: i128) -> i128 {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo) as u128 + 1;
        let r = (u128::from(self.next_u64()) << 64 | u128::from(self.next_u64())) % span;
        lo + r as i128
    }

    /// True with probability `num / den`: `below(den) < num`.
    pub fn chance(&mut self, num: u64, den: u64) -> bool {
        self.below(den) < num
    }

    /// `xs[below(len)]`.
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> Option<&'a T> {
        xs.get(usize::try_from(self.below(xs.len() as u64)).unwrap_or(0))
    }
}

/// The seed of run `index` of the check `name`: FNV-1a of the name, mixed
/// with the base seed, then one SplitMix64 step.
pub fn seed_for(base: u64, name: &str, index: u64) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in name.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    let mut r = Rng::new(h ^ base.rotate_left(17) ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    r.next_u64()
}
