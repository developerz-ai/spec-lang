// The spec-lang runtime of the TypeScript printer: `Int`.
// `Int` is exact: a `bigint`, never a JS number, so `+ - *` and comparisons are the native operators and no
// result is ever rounded or wrapped. What the native operators do not mean is here.

import { fail, stop } from "./failure.ts";

export type Int = bigint;

/** Floored modulo: the result has the sign of the divisor (`-1 mod 4 == 3`). */
export function mod(a: Int, b: Int): Int {
  if (b === 0n) fail("DivByZero");
  const r = a % b;
  return r !== 0n && r < 0n !== b < 0n ? r + b : r;
}

/** `a ^ b`; a negative exponent has no Int result. */
export function pow(a: Int, b: Int): Int {
  if (b < 0n) stop("E-TYPE-MISMATCH", "a negative exponent");
  return a ** b;
}

/** `/` on Int is an open choice: the program says how to round. */
export function div(_a: Int, _b: Int): never {
  stop("E-OPEN", "`/` on Int is an open choice: write `a.div(b, floor)` or `a.div(b, trunc)`");
}

function natural(op: string, a: Int, b: Int): void {
  if (a < 0n || b < 0n) stop("E-TYPE-MISMATCH", `\`${op}\` on a negative Int`);
}

export function band(a: Int, b: Int): Int {
  natural("band", a, b);
  return a & b;
}

export function bor(a: Int, b: Int): Int {
  natural("bor", a, b);
  return a | b;
}

export function bxor(a: Int, b: Int): Int {
  natural("bxor", a, b);
  return a ^ b;
}

/** `a * 2^b`. */
export function shl(a: Int, b: Int): Int {
  natural("shl", a, b);
  return a << b;
}

/** `floor(a / 2^b)`. */
export function shr(a: Int, b: Int): Int {
  natural("shr", a, b);
  return a >> b;
}

export function abs(a: Int): Int {
  return a < 0n ? -a : a;
}

export function minInt(a: Int, b: Int): Int {
  return a <= b ? a : b;
}

export function maxInt(a: Int, b: Int): Int {
  return a >= b ? a : b;
}

/** The value pulled into `lo..=hi`. */
export function clamp(a: Int, lo: Int, hi: Int): Int {
  if (lo > hi) stop("E-TYPE-MISMATCH", "`clamp` with lo above hi");
  return a < lo ? lo : a > hi ? hi : a;
}
