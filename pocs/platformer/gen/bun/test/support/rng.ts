// The checks' generator: SplitMix64 and the seed of each run, bit for bit the engine's, so a property or a law
// here draws the inputs `spec-lang test` draws for the same seed.

const MASK = (1n << 64n) - 1n;
const GOLDEN = 0x9e3779b97f4a7c15n;

export class Rng {
  private state: bigint;

  constructor(seed: bigint) {
    this.state = seed & MASK;
  }

  next(): bigint {
    this.state = (this.state + GOLDEN) & MASK;
    let z = this.state;
    z = ((z ^ (z >> 30n)) * 0xbf58476d1ce4e5b9n) & MASK;
    z = ((z ^ (z >> 27n)) * 0x94d049bb133111ebn) & MASK;
    return z ^ (z >> 31n);
  }

  /** Uniform in `0..n` (`0` when `n` is 0): `next() % n`. */
  below(n: bigint): bigint {
    return n === 0n ? 0n : this.next() % n;
  }

  /** In `lo..=hi`: two draws make a 128-bit number, high first, modulo the span. */
  between(lo: bigint, hi: bigint): bigint {
    if (hi <= lo) return lo;
    const span = hi - lo + 1n;
    const high = this.next();
    const r = ((high << 64n) | this.next()) % span;
    return lo + r;
  }

  /** True with probability `num / den`. */
  chance(num: bigint, den: bigint): boolean {
    return this.below(den) < num;
  }

  pick<T>(xs: readonly T[]): T | undefined {
    return xs[Number(this.below(BigInt(xs.length)))];
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
