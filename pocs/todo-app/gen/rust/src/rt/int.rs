//! `Int` operations that can leave the range or have a domain: each stops instead of wrapping.

use super::{Halt, Int, R, fail, overflow, stop};

fn mismatch(msg: &str) -> Halt {
    stop("E-TYPE-MISMATCH", msg)
}

pub fn add(a: Int, b: Int) -> R<Int> {
    a.checked_add(b).ok_or_else(overflow)
}

pub fn sub(a: Int, b: Int) -> R<Int> {
    a.checked_sub(b).ok_or_else(overflow)
}

pub fn mul(a: Int, b: Int) -> R<Int> {
    a.checked_mul(b).ok_or_else(overflow)
}

pub fn neg(a: Int) -> R<Int> {
    a.checked_neg().ok_or_else(overflow)
}

/// Floored: the result has the sign of the divisor; divisor 0 fails `DivByZero`.
pub fn modulo(a: Int, b: Int) -> R<Int> {
    if b == 0 {
        return Err(fail("DivByZero", Vec::new()));
    }
    a.checked_rem_euclid(b).map(|m| if b < 0 && m != 0 { m + b } else { m }).ok_or_else(overflow)
}

pub fn pow(a: Int, b: Int) -> R<Int> {
    if b < 0 {
        return Err(mismatch("`pow` with a negative exponent"));
    }
    u32::try_from(b).ok().and_then(|e| a.checked_pow(e)).ok_or_else(overflow)
}

/// `/` on Int: an open choice, never a guess.
pub fn div(_: Int, _: Int) -> R<Int> {
    Err(stop("E-OPEN", "`/` on Int is an open choice: write `a.div(b, floor)` or `a.div(b, trunc)`"))
}

fn naturals(a: Int, b: Int, op: &str) -> R<()> {
    if a < 0 || b < 0 { Err(mismatch(&format!("`{op}` on a negative Int"))) } else { Ok(()) }
}

pub fn band(a: Int, b: Int) -> R<Int> {
    naturals(a, b, "band")?;
    Ok(a & b)
}

pub fn bor(a: Int, b: Int) -> R<Int> {
    naturals(a, b, "bor")?;
    Ok(a | b)
}

pub fn bxor(a: Int, b: Int) -> R<Int> {
    naturals(a, b, "bxor")?;
    Ok(a ^ b)
}

/// `a * 2^b`.
pub fn shl(a: Int, b: Int) -> R<Int> {
    naturals(a, b, "shl")?;
    u32::try_from(b).ok().and_then(|s| a.checked_shl(s).filter(|v| v >> s == a)).ok_or_else(overflow)
}

/// `floor(a / 2^b)`; 0 past the width.
pub fn shr(a: Int, b: Int) -> R<Int> {
    naturals(a, b, "shr")?;
    Ok(u32::try_from(b).ok().and_then(|s| a.checked_shr(s)).unwrap_or(0))
}

pub fn abs(a: Int) -> R<Int> {
    a.checked_abs().ok_or_else(|| mismatch("no `abs` of this Int"))
}

pub fn clamp(a: Int, lo: Int, hi: Int) -> R<Int> {
    if lo > hi {
        return Err(mismatch("`clamp` with lo > hi"));
    }
    Ok(a.clamp(lo, hi))
}

/// A count as an `Int`.
pub fn count(n: usize) -> Int {
    Int::try_from(n).unwrap_or(Int::MAX)
}
