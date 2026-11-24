// The spec-lang runtime of the TypeScript printer: `Option`, structural equality, and the List operations the
// native array methods do not already mean.

import { OrdMap, OrdSet, Range } from "./collections.ts";
import { fail, Failure, missing, stop } from "./failure.ts";
import type { Int } from "./int.ts";
import type { Type } from "./types.ts";

export type Option<T> = { readonly kind: "Some"; readonly value: T } | { readonly kind: "none" };

export const none: Option<never> = Object.freeze({ kind: "none" });

export function some<T>(value: T): Option<T> {
  return { kind: "Some", value };
}

/** `opt else fallback` on an Option: `Some(v)` gives `v`, `none` the fallback. */
export function someOr<T>(value: Option<T>, fallback: () => T): T {
  return value.kind === "Some" ? value.value : fallback();
}

/** `opt else fallback` where `opt` may also fail: a failure or `none` give the fallback. */
export function someOrElse<T>(value: () => Option<T>, fallback: () => T): T {
  let v: Option<T>;
  try {
    v = value();
  } catch (e) {
    if (e instanceof Failure) return fallback();
    throw e;
  }
  return v.kind === "Some" ? v.value : fallback();
}

function equalItems(a: readonly unknown[], b: readonly unknown[]): boolean {
  return a.length === b.length && a.every((x, i) => equal(x, b[i]));
}

/** Structural `==` on every type but functions. */
export function equal(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (typeof a !== "object" || typeof b !== "object" || a === null || b === null) return false;
  if (Array.isArray(a)) return Array.isArray(b) && equalItems(a, b);
  if (a instanceof OrdSet) return b instanceof OrdSet && equalItems(a.items, b.items);
  if (a instanceof OrdMap) {
    return b instanceof OrdMap && equalItems(a.keyList, b.keyList) && equalItems(a.valueList, b.valueList);
  }
  if (a instanceof Range) return b instanceof Range && a.lo === b.lo && a.hi === b.hi && a.inclusive === b.inclusive;
  const keys = Object.keys(a);
  if (Object.keys(b).length !== keys.length) return false;
  return keys.every((k) => equal((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k]));
}

/** An index as a JS number, or -1 when it is not one of `xs`. */
function index(xs: readonly unknown[], i: Int): number {
  return i >= 0n && i < BigInt(xs.length) ? Number(i) : -1;
}

/** `xs[i]`: outside the list fails `Missing`. */
export function at<T>(xs: readonly T[], i: Int): T {
  const n = index(xs, i);
  return n < 0 ? missing() : (xs[n] as T);
}

/** `xs.get(i)`. */
export function get<T>(xs: readonly T[], i: Int): Option<T> {
  const n = index(xs, i);
  return n < 0 ? none : some(xs[n] as T);
}

/** `xs[i] = v`. */
export function setAt<T>(xs: readonly T[], i: Int, v: T): readonly T[] {
  const n = index(xs, i);
  return n < 0 ? missing() : xs.with(n, v);
}

export function first<T>(xs: readonly T[]): T {
  return xs.length === 0 ? missing() : (xs[0] as T);
}

export function last<T>(xs: readonly T[]): T {
  return xs.length === 0 ? missing() : (xs[xs.length - 1] as T);
}

export function find<T>(xs: readonly T[], holds: (x: T) => boolean): Option<T> {
  for (const x of xs) if (holds(x)) return some(x);
  return none;
}

export function position<T>(xs: readonly T[], holds: (x: T) => boolean): Option<Int> {
  for (let i = 0; i < xs.length; i++) if (holds(xs[i] as T)) return some(BigInt(i));
  return none;
}

export function sum(xs: readonly Int[]): Int {
  let total = 0n;
  for (const x of xs) total += x;
  return total;
}

export function sumBy<T>(xs: readonly T[], f: (x: T) => Int): Int {
  let total = 0n;
  for (const x of xs) total += f(x);
  return total;
}

/** The item with the least (`sign` -1) or greatest (1) key; the earliest of ties; empty fails `Missing`. */
function best<T, K>(xs: readonly T[], key: (x: T) => K, type: Type<K>, sign: number): T {
  let found: { readonly key: K; readonly item: T } | undefined;
  for (const x of xs) {
    const k = key(x);
    if (found === undefined || type.compare(k, found.key) * sign > 0) found = { key: k, item: x };
  }
  return found === undefined ? missing() : found.item;
}

export function min<T>(xs: readonly T[], type: Type<T>): T {
  return best(xs, (x) => x, type, -1);
}

export function max<T>(xs: readonly T[], type: Type<T>): T {
  return best(xs, (x) => x, type, 1);
}

export function minBy<T, K>(xs: readonly T[], key: (x: T) => K, type: Type<K>): T {
  return best(xs, key, type, -1);
}

export function maxBy<T, K>(xs: readonly T[], key: (x: T) => K, type: Type<K>): T {
  return best(xs, key, type, 1);
}

export function contains<T>(xs: readonly T[], x: T): boolean {
  return xs.some((y) => equal(x, y));
}

/** The earliest of each group of equal items, in order. */
export function distinct<T>(xs: readonly T[], type: Type<T>): readonly T[] {
  let seen = OrdSet.empty(type);
  return xs.filter((x) => {
    if (seen.has(x)) return false;
    seen = seen.insert(x);
    return true;
  });
}

/** A stable sort by the key's value order; `desc` reverses the order, not the ties. */
export function sortBy<T, K>(xs: readonly T[], key: (x: T) => K, type: Type<K>, desc = false): readonly T[] {
  const keyed = xs.map((item) => ({ key: key(item), item }));
  const sign = desc ? -1 : 1;
  return keyed.sort((a, b) => type.compare(a.key, b.key) * sign).map((k) => k.item);
}

export function sort<T>(xs: readonly T[], type: Type<T>, desc = false): readonly T[] {
  return sortBy(xs, (x) => x, type, desc);
}

function count(n: Int, op: string): number {
  if (n < 0n) stop("E-TYPE-ARITY", `\`${op}\` takes one Int, at least 0`);
  return n > 2n ** 32n ? 2 ** 32 : Number(n);
}

export function take<T>(xs: readonly T[], n: Int): readonly T[] {
  return xs.slice(0, count(n, "take"));
}

export function drop<T>(xs: readonly T[], n: Int): readonly T[] {
  return xs.slice(count(n, "drop"));
}

/** Pairs by position; unequal sizes fail `Uneven`. */
export function zip<T, U>(xs: readonly T[], ys: readonly U[]): readonly (readonly [T, U])[] {
  if (xs.length !== ys.length) fail("Uneven");
  return xs.map((x, i) => [x, ys[i] as U] as const);
}

/** Items by key, the order kept inside a group. */
export function groupBy<T, K>(xs: readonly T[], key: (x: T) => K, type: Type<K>): OrdMap<K, readonly T[]> {
  let groups = OrdMap.empty<K, readonly T[]>(type);
  for (const x of xs) {
    const k = key(x);
    const group = groups.get(k);
    groups = groups.set(k, group.kind === "Some" ? [...group.value, x] : [x]);
  }
  return groups;
}

/** `x` put before index `at` (0 to size). */
export function insert<T>(xs: readonly T[], at: Int, x: T): readonly T[] {
  if (at < 0n || at > BigInt(xs.length)) missing();
  return xs.toSpliced(Number(at), 0, x);
}

/** The item at `at` removed. */
export function removeAt<T>(xs: readonly T[], at: Int): readonly T[] {
  const n = index(xs, at);
  return n < 0 ? missing() : xs.toSpliced(n, 1);
}

/** `x in lo..=hi` and the like, as a value. */
export function range(lo: Int, hi: Int, inclusive: boolean): Range {
  return new Range(lo, hi, inclusive);
}
