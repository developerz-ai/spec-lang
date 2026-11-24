// Projected by hand from the shot hash of contract section 2 (docs/contract.md). Stand-in for spec-lang build output; change the spec, not this file.

const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
const PRIME: u64 = 0x0000_0100_0000_01b3;

/// FNV-1a, 64 bits.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes
        .iter()
        .fold(OFFSET, |h, b| (h ^ u64::from(*b)).wrapping_mul(PRIME))
}
