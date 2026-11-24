// The `game` library's runtime for TypeScript (libs/game/real/ts): the pixel hash `shot` prints. Lifted from
// pocs/platformer/out/bun/src/app/hash.ts.

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
