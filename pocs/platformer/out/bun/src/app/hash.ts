// Projected by hand from the pixel hash of shot: FNV-1a 64 (docs/game-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

const PRIME_LOW = 0x1b3;

/**
 * FNV-1a 64 (offset 0xcbf29ce484222325, prime 0x100000001b3) as 16 lower-case hex digits.
 * The hash is held as two 32-bit halves. The prime is 2^40 + 0x1b3, so h * prime mod 2^64 is
 *   low'  = low * 0x1b3                                  (below 2^41: exact in a double)
 *   high' = high * 0x1b3 + (low << 8) + carry of low'    (each term below 2^41: exact)
 * No BigInt is allocated per byte.
 */
export function fnv1a64(bytes: Uint8Array): string {
  let high = 0xcbf29ce4;
  let low = 0x84222325;
  for (let i = 0; i < bytes.length; i++) {
    low = (low ^ (bytes[i] as number)) >>> 0;
    const lowProduct = low * PRIME_LOW;
    const carry = Math.floor(lowProduct / 0x100000000);
    high = (high * PRIME_LOW + ((low << 8) >>> 0) + carry) >>> 0;
    low = lowProduct >>> 0;
  }
  return high.toString(16).padStart(8, "0") + low.toString(16).padStart(8, "0");
}
