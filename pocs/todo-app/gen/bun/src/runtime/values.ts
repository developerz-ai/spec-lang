// Runtime shim of the bun backend of spec-lang build: claims, order, equality, Set and Map. The same in every generated project; change the backend, not this file.

/** A base value that passed a claim: `type Title = Text where is_title(self)`. */
export type Brand<Base, Name extends string> = Base & { readonly __brand: Name };

/** A value outside its type reached a place of that type. Edges check first, so this is a bug in the program. */
export class ClaimViolation extends Error {}

/** `map[key]` on a key the map does not hold. */
export class Missing extends Error {}

export interface Claimed<Base, T extends Base> {
  readonly name: string;
  /** At an edge (an argument, a store): the value, or undefined when the claim does not hold. */
  readonly parse: (value: Base) => T | undefined;
  /** Inside the program: the value, checked; a failure is a bug. */
  readonly of: (value: Base) => T;
}

export function claimed<Base, T extends Base>(name: string, holds: (self: Base) => boolean): Claimed<Base, T> {
  const parse = (value: Base): T | undefined => (holds(value) ? (value as T) : undefined);
  return Object.freeze({
    name,
    parse,
    of: (value: Base): T => {
      const checked = parse(value);
      if (checked === undefined) throw new ClaimViolation(`not a ${name}: ${String(value)}`);
      return checked;
    },
  });
}

/** Order on Int, Text (by code point) and tuples of them. */
export function compare(a: unknown, b: unknown): number {
  if (typeof a === "string" && typeof b === "string") return compareText(a, b);
  if (Array.isArray(a) && Array.isArray(b)) {
    for (let i = 0; i < a.length && i < b.length; i++) {
      const d = compare(a[i], b[i]);
      if (d !== 0) return d;
    }
    return a.length - b.length;
  }
  if ((typeof a === "bigint" || typeof a === "number") && (typeof b === "bigint" || typeof b === "number")) {
    return a < b ? -1 : a > b ? 1 : 0;
  }
  throw new TypeError("values of this type have no order");
}

/** UTF-16 order differs from code-point order only where a surrogate meets a unit above U+DFFF. */
function compareText(a: string, b: string): number {
  if (a === b) return 0;
  const n = Math.min(a.length, b.length);
  for (let i = 0; i < n; i++) {
    const x = a.charCodeAt(i);
    const y = b.charCodeAt(i);
    if (x !== y) return fix(x) - fix(y);
  }
  return a.length - b.length;
}

function fix(unit: number): number {
  return unit >= 0xd800 && unit <= 0xdfff ? unit + 0x2000 : unit >= 0xe000 ? unit - 0x800 : unit;
}

/** `xs.sort(key)`: ascending by key, stable. */
export function sortBy<T>(items: readonly T[], key: (item: T) => unknown): T[] {
  return items
    .map((item) => ({ item, key: key(item) }))
    .sort((a, b) => compare(a.key, b.key))
    .map((pair) => pair.item);
}

/** A `Set`: a frozen array, ascending and without repeats, so `==` is element-wise. */
export function setOf<T>(items: Iterable<T>): readonly T[] {
  const sorted = [...items].sort(compare);
  return Object.freeze(sorted.filter((item, i) => i === 0 || compare(sorted[i - 1], item) !== 0));
}

/** `map[key]`. */
export function at<K, V>(map: ReadonlyMap<K, V>, key: K): V {
  const value = map.get(key);
  if (value === undefined) throw new Missing(String(key));
  return value;
}

/**
 * `map[key] = value`. A `Map` iterates in ascending key order, and a JS Map in insertion order,
 * so the two are kept equal: a key above every other is appended, any other new key re-sorts.
 */
export function put<K, V>(map: Map<K, V>, key: K, value: V): void {
  if (map.has(key)) {
    map.set(key, value);
    return;
  }
  let last: K | undefined;
  for (const k of map.keys()) last = k;
  map.set(key, value);
  if (last === undefined || compare(last, key) < 0) return;
  const entries = [...map].sort((a, b) => compare(a[0], b[0]));
  map.clear();
  for (const [k, v] of entries) map.set(k, v);
}

const owned = new WeakSet<object>();

/** The store session read this world for one rule call and holds no other use of it: its maps may be changed in place. */
export function own<W>(world: W): W {
  if (typeof world === "object" && world !== null) {
    for (const value of Object.values(world)) if (value instanceof Map) owned.add(value);
  }
  return world;
}

/**
 * The map a rule body writes. A rule is atomic and the process commits once, so a map the caller
 * handed over (`own`) is the draft itself; any other caller keeps its world and the draft is a copy.
 */
export function draft<K, V>(map: ReadonlyMap<K, V>): Map<K, V> {
  return owned.delete(map) ? (map as Map<K, V>) : new Map(map);
}

/** `Map(k -> v, ...)`, in ascending key order. */
export function mapOf<K, V>(entries: Iterable<readonly [K, V]>): Map<K, V> {
  return new Map([...entries].sort((a, b) => compare(a[0], b[0])));
}

/** `record.with(field: value)`. */
export function update<T extends object>(record: T, patch: Partial<T>): T {
  return Object.freeze({ ...record, ...patch });
}

/** Structural `==` on every type. */
export function equals(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (typeof a !== "object" || typeof b !== "object" || a === null || b === null) return false;
  if (Array.isArray(a)) return Array.isArray(b) && a.length === b.length && a.every((x, i) => equals(x, b[i]));
  if (a instanceof Map) {
    if (!(b instanceof Map) || a.size !== b.size) return false;
    for (const [k, v] of a) if (!b.has(k) || !equals(v, b.get(k))) return false;
    return true;
  }
  const keys = Object.keys(a);
  if (Array.isArray(b) || b instanceof Map || Object.keys(b).length !== keys.length) return false;
  return keys.every((k) => equals((a as Record<string, unknown>)[k], (b as Record<string, unknown>)[k]));
}
