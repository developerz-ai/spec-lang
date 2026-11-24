// The spec-lang runtime of the TypeScript printer: `Set`, `Map` and ranges.
// A Set and a Map are immutable values kept in ascending value order, so they iterate the way the language
// says and two equal ones hold equal arrays. Every change gives a new one.

import { missing, stop } from "./failure.ts";
import type { Int } from "./int.ts";
import type { Type } from "./types.ts";
import { none, type Option, some } from "./values.ts";

/** The index of `x` in ascending `items`, or `-(insertion point) - 1`. */
function search<T>(type: Type<T>, items: readonly T[], x: T): number {
  let lo = 0;
  let hi = items.length - 1;
  while (lo <= hi) {
    const mid = (lo + hi) >> 1;
    const d = type.compare(items[mid] as T, x);
    if (d === 0) return mid;
    if (d < 0) lo = mid + 1;
    else hi = mid - 1;
  }
  return -lo - 1;
}

export class OrdSet<T> implements Iterable<T> {
  private constructor(
    readonly type: Type<T>,
    /** Ascending, without repeats. */
    readonly items: readonly T[],
  ) {}

  static empty<T>(type: Type<T>): OrdSet<T> {
    return new OrdSet(type, []);
  }

  /** The set of the items: one of each, ascending. */
  static of<T>(type: Type<T>, items: Iterable<T>): OrdSet<T> {
    const sorted = [...items].sort((a, b) => type.compare(a, b));
    return new OrdSet(
      type,
      sorted.filter((x, i) => i === 0 || type.compare(sorted[i - 1] as T, x) !== 0),
    );
  }

  [Symbol.iterator](): Iterator<T> {
    return this.items[Symbol.iterator]();
  }

  get size(): Int {
    return BigInt(this.items.length);
  }

  has(x: T): boolean {
    return search(this.type, this.items, x) >= 0;
  }

  insert(x: T): OrdSet<T> {
    const at = search(this.type, this.items, x);
    if (at >= 0) return this;
    return new OrdSet(this.type, this.items.toSpliced(-at - 1, 0, x));
  }

  remove(x: T): OrdSet<T> {
    const at = search(this.type, this.items, x);
    return at < 0 ? this : new OrdSet(this.type, this.items.toSpliced(at, 1));
  }

  union(other: OrdSet<T>): OrdSet<T> {
    return OrdSet.of(this.type, [...this.items, ...other.items]);
  }

  difference(other: OrdSet<T>): OrdSet<T> {
    return new OrdSet(
      this.type,
      this.items.filter((x) => !other.has(x)),
    );
  }

  /** The elements in ascending order. */
  list(): readonly T[] {
    return this.items;
  }
}

export class OrdMap<K, V> implements Iterable<readonly [K, V]> {
  private constructor(
    readonly keyType: Type<K>,
    /** Ascending, without repeats. */
    readonly keyList: readonly K[],
    /** `valueList[i]` is the value at `keyList[i]`. */
    readonly valueList: readonly V[],
  ) {}

  static empty<K, V>(keyType: Type<K>): OrdMap<K, V> {
    return new OrdMap<K, V>(keyType, [], []);
  }

  /** Entries left to right; a later equal key replaces an earlier one. */
  static of<K, V>(keyType: Type<K>, entries: Iterable<readonly [K, V]>): OrdMap<K, V> {
    let m = OrdMap.empty<K, V>(keyType);
    for (const [k, v] of entries) m = m.set(k, v);
    return m;
  }

  *[Symbol.iterator](): Iterator<readonly [K, V]> {
    for (let i = 0; i < this.keyList.length; i++) yield [this.keyList[i] as K, this.valueList[i] as V];
  }

  get size(): Int {
    return BigInt(this.keyList.length);
  }

  has(k: K): boolean {
    return search(this.keyType, this.keyList, k) >= 0;
  }

  get(k: K): Option<V> {
    const at = search(this.keyType, this.keyList, k);
    return at < 0 ? none : some(this.valueList[at] as V);
  }

  /** `m[k]`: the value at `k`; an absent key fails `Missing`. */
  at(k: K): V {
    const at = search(this.keyType, this.keyList, k);
    return at < 0 ? missing() : (this.valueList[at] as V);
  }

  /** `m[k] = v`: adds or replaces. */
  set(k: K, v: V): OrdMap<K, V> {
    const at = search(this.keyType, this.keyList, k);
    if (at >= 0) return new OrdMap(this.keyType, this.keyList, this.valueList.with(at, v));
    const i = -at - 1;
    return new OrdMap(this.keyType, this.keyList.toSpliced(i, 0, k), this.valueList.toSpliced(i, 0, v));
  }

  remove(k: K): OrdMap<K, V> {
    const at = search(this.keyType, this.keyList, k);
    if (at < 0) return this;
    return new OrdMap(this.keyType, this.keyList.toSpliced(at, 1), this.valueList.toSpliced(at, 1));
  }

  /** Keys ascending. */
  keys(): readonly K[] {
    return this.keyList;
  }

  /** Values in ascending key order. */
  values(): readonly V[] {
    return this.valueList;
  }
}

/** `lo..=hi` or `lo..<hi` as a value. */
export class Range {
  constructor(
    readonly lo: Int,
    readonly hi: Int,
    readonly inclusive: boolean,
  ) {}

  has(n: Int): boolean {
    return n >= this.lo && (this.inclusive ? n <= this.hi : n < this.hi);
  }

  /** The numbers ascending; more than ten million stops. */
  list(): readonly Int[] {
    const end = this.inclusive ? this.hi : this.hi - 1n;
    if (end - this.lo > 10_000_000n) stop("E-RUNTIME", "a range of more than ten million numbers");
    const out: Int[] = [];
    for (let n = this.lo; n <= end; n++) out.push(n);
    return out;
  }
}
