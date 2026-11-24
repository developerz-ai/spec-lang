// The checks' generator: SplitMix64 and the seed of each run, bit for bit the engine's, so a property or a law
// here draws the inputs `spec-lang test` draws for the same seed.

const MASK = (1n << 64n) - 1n;
const GOLDEN = 0x9e3779b97f4a7c15n;

/** The high 32 bits of the product of two unsigned 32-bit numbers. */
function mulHigh(a: number, b: number): number {
  const al = a & 0xffff;
  const ah = a >>> 16;
  const bl = b & 0xffff;
  const bh = b >>> 16;
  const mid = ah * bl + ((al * bl) >>> 16);
  const cross = al * bh + (mid & 0xffff);
  return (ah * bh + Math.floor(mid / 0x10000) + Math.floor(cross / 0x10000)) >>> 0;
}

/** SplitMix64, its 64 bits kept as two 32-bit halves: a BigInt is made only where a draw needs all of them. */
export class Rng {
  private stateHigh: number;
  private stateLow: number;
  private high = 0;
  private low = 0;

  constructor(seed: bigint) {
    this.stateHigh = Number((seed >> 32n) & 0xffffffffn);
    this.stateLow = Number(seed & 0xffffffffn);
  }

  /** `(high, low) ^= (high, low) >> by; (high, low) *= (mh, ml)`, modulo 2^64. */
  private mix(by: number, mh: number, ml: number): void {
    const low = (this.low ^ ((this.low >>> by) | (this.high << (32 - by)))) >>> 0;
    const high = (this.high ^ (this.high >>> by)) >>> 0;
    this.low = Math.imul(low, ml) >>> 0;
    this.high = (mulHigh(low, ml) + Math.imul(low, mh) + Math.imul(high, ml)) >>> 0;
  }

  private step(): void {
    const low = this.stateLow + 0x7f4a7c15;
    this.stateHigh = (this.stateHigh + 0x9e3779b9 + (low > 0xffffffff ? 1 : 0)) >>> 0;
    this.stateLow = low >>> 0;
    this.high = this.stateHigh;
    this.low = this.stateLow;
    this.mix(30, 0xbf58476d, 0x1ce4e5b9);
    this.mix(27, 0x94d049bb, 0x133111eb);
    this.low = (this.low ^ ((this.low >>> 31) | (this.high << 1))) >>> 0;
    this.high = (this.high ^ (this.high >>> 31)) >>> 0;
  }

  next(): bigint {
    this.step();
    return (BigInt(this.high) << 32n) | BigInt(this.low);
  }

  /** Uniform in `0..n` (`0` when `n` is 0): `next() % n`; `n` is at most 2^20. */
  below(n: number): number {
    if (n === 0) return 0;
    this.step();
    return ((this.high % n) * 0x100000000 + this.low) % n;
  }

  /** In `lo..=hi`: two draws make a 128-bit number, high first, modulo the span. */
  between(lo: bigint, hi: bigint): bigint {
    if (hi <= lo) return lo;
    const span = hi - lo + 1n;
    if (span <= 0xffffffffn) {
      const n = Number(span);
      this.step();
      const high = this.mod(0, n);
      this.step();
      return lo + BigInt(this.mod(high, n));
    }
    const high = this.next();
    return lo + (((high << 64n) | this.next()) % span);
  }

  /** `(r * 2^64 + drawn) % n`, sixteen bits at a time so every step is exact; `r < n < 2^32`. */
  private mod(r: number, n: number): number {
    let m = (r * 0x10000 + (this.high >>> 16)) % n;
    m = (m * 0x10000 + (this.high & 0xffff)) % n;
    m = (m * 0x10000 + (this.low >>> 16)) % n;
    return (m * 0x10000 + (this.low & 0xffff)) % n;
  }

  /** True with probability `num / den`. */
  chance(num: number, den: number): boolean {
    return this.below(den) < num;
  }

  pick<T>(xs: readonly T[]): T | undefined {
    return xs[this.below(xs.length)];
  }
}

const encoder = new TextEncoder();

function rotateLeft(x: bigint, n: bigint): bigint {
  return ((x << n) | (x >> (64n - n))) & MASK;
}

/** The seed of run `index` of the check `name`: FNV-1a of the name, mixed with the base seed, one SplitMix64 step. */
export function seedFor(base: bigint, name: string, index: number): bigint {
  let h = 0xcbf29ce484222325n;
  for (const b of encoder.encode(name)) {
    h ^= BigInt(b);
    h = (h * 0x100000001b3n) & MASK;
  }
  return new Rng(h ^ rotateLeft(base, 17n) ^ ((BigInt(index) * GOLDEN) & MASK)).next();
}
