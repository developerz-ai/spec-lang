// Runtime shim of the bun backend of spec-lang build: store codec. The same in every generated project; change the backend, not this file.

import { compare } from "../values.ts";
import { type Json, readJson } from "./json-reader.ts";

/** The store does not hold this value: an Int past 2^53. Reported as `full`. */
class Full extends Error {}

/** The JSON is valid but is not a world of this program. */
class Unfit extends Error {}

function unfit(why: string): never {
  throw new Unfit(why);
}

/** The largest magnitude of a stored Int: a backend constraint carried as a refusal, not a language rule. */
export const MAX_INT = 2n ** 53n;

/** Where a fast read stands in the store text. */
interface Cursor {
  at: number;
}

/**
 * The fast path: reads the exact bytes `encode` writes, straight from the text, with no JSON tree.
 * It never decides that a store is unreadable: on anything else it gives `MISS`, and the strict reader runs.
 */
interface Fast<T> {
  /** A regular expression source for the canonical bytes of one value, without capture groups. */
  readonly shape: string | undefined;
  /** The value from text that matched `shape`. */
  readonly read: ((raw: string) => T | typeof MISS) | undefined;
  /** The value at the cursor, which moves past it. */
  readonly readAt: (text: string, cursor: Cursor) => T | typeof MISS;
}

const MISS: unique symbol = Symbol("not canonical");

/** One type's place in the store file: a strict reader, the canonical writer, and a fast reader of what it wrote. */
export interface Codec<T> {
  readonly decode: (json: Json) => T;
  readonly encode: (value: T) => string;
  readonly fast: Fast<T>;
}

/** A value whose canonical bytes one regular expression recognises. */
function leaf<T>(shape: string, read: (raw: string) => T | typeof MISS): Fast<T> {
  const sticky = new RegExp(shape, "y");
  return {
    shape,
    read,
    readAt: (text, cursor) => {
      sticky.lastIndex = cursor.at;
      const m = sticky.exec(text);
      if (m === null) return MISS;
      cursor.at = sticky.lastIndex;
      return read(m[0]);
    },
  };
}

function literal(text: string, cursor: Cursor, expected: string): boolean {
  if (!text.startsWith(expected, cursor.at)) return false;
  cursor.at += expected.length;
  return true;
}

/** An Int: a JSON number with no fraction or exponent, within 2^53, then the type's claim. */
export function int<T extends bigint>(parse: (n: bigint) => T | undefined): Codec<T> {
  return {
    decode: (json) => {
      if (json.kind !== "number" || json.integer === undefined) unfit("not an integer");
      if (json.integer > MAX_INT || json.integer < -MAX_INT) unfit("integer out of range");
      return parse(json.integer) ?? unfit("integer outside its type");
    },
    encode: (value) => {
      if (value > MAX_INT || value < -MAX_INT) throw new Full();
      return String(value);
    },
    fast: leaf("-?(?:0|[1-9][0-9]{0,15})", (raw) => {
      const n = BigInt(raw);
      return n > MAX_INT || n < -MAX_INT ? MISS : (parse(n) ?? MISS);
    }),
  };
}

// biome-ignore lint/suspicious/noControlCharactersInRegex: JSON requires these escaped.
const ESCAPE = /["\\\x00-\x1f]/g;

/** Only `"`, `\` and what JSON cannot hold raw are escaped; everything else is raw UTF-8. */
function quote(s: string): string {
  return `"${s.replace(ESCAPE, (c) => (c === '"' || c === "\\" ? `\\${c}` : `\\u${c.charCodeAt(0).toString(16).padStart(4, "0")}`))}"`;
}

/** A JSON string holding nothing but raw characters and the two escapes the writer emits. */
const PLAIN_STRING = '"(?:[^"\\\\\\x00-\\x1f]|\\\\["\\\\])*"';
const UNESCAPE = /\\(["\\])/g;

/** A Text, then the type's claim. */
export function text<T extends string>(parse: (s: string) => T | undefined): Codec<T> {
  return {
    decode: (json) => {
      if (json.kind !== "string") unfit("not a string");
      return parse(json.value) ?? unfit("text outside its type");
    },
    encode: quote,
    fast: leaf(PLAIN_STRING, (raw) => {
      const body = raw.slice(1, -1);
      return parse(body.includes("\\") ? body.replace(UNESCAPE, "$1") : body) ?? MISS;
    }),
  };
}

export const bool: Codec<boolean> = {
  decode: (json) => (json.kind === "bool" ? json.value : unfit("not a boolean")),
  encode: String,
  fast: leaf("true|false", (raw) => raw === "true"),
};

/** A sum with no data: the variant's name. */
export function variant<T extends string>(names: readonly T[]): Codec<T> {
  return {
    decode: (json) => {
      if (json.kind !== "string") unfit("not a string");
      return names.find((n) => n === json.value) ?? unfit("not a variant");
    },
    encode: (value) => `"${value}"`,
    fast: fastVariant(names),
  };
}

function fastVariant<T extends string>(names: readonly T[]): Fast<T> {
  const byRaw: ReadonlyMap<string, T> = new Map(names.map((n) => [`"${n}"`, n]));
  return leaf(`"(?:${names.join("|")})"`, (raw) => byRaw.get(raw) ?? MISS);
}

const EMPTY: readonly never[] = Object.freeze([]);

/** `a < b` for keys: the operator itself on Ints, code-point order on Texts. */
function ascending(a: unknown, b: unknown): boolean {
  return typeof a === "bigint" && typeof b === "bigint" ? a < b : compare(a, b) < 0;
}

/** A Set: an array in any order with no value twice; written ascending. */
export function set<T>(of: Codec<T>): Codec<readonly T[]> {
  return {
    decode: (json) => {
      if (json.kind !== "array") unfit("not an array");
      const items = json.items.map(of.decode);
      // The canonical writer sorts, so strictly ascending is the common case: no repeat is then possible.
      if (items.every((item, i) => i === 0 || compare(items[i - 1], item) < 0)) return Object.freeze(items);
      const sorted = [...items].sort(compare);
      if (sorted.some((item, i) => i > 0 && compare(sorted[i - 1], item) === 0)) unfit("value twice");
      return Object.freeze(sorted);
    },
    encode: (value) => `[${value.map(of.encode).join(",")}]`,
    fast: fastSet(of.fast),
  };
}

/** The writer sorts a Set, so strictly ascending items are the only canonical form, and it rules out repeats. */
function fastSet<T>(of: Fast<T>): Fast<readonly T[]> {
  const { shape, read } = of;
  if (shape === undefined || read === undefined) return { shape: undefined, read: undefined, readAt: () => MISS };
  const item = new RegExp(shape, "y");
  return leaf(`\\[(?:${shape}(?:,${shape})*)?\\]`, (raw) => {
    if (raw.length === 2) return EMPTY;
    const items: T[] = [];
    for (let at = 1; at < raw.length - 1; at = item.lastIndex + 1) {
      item.lastIndex = at;
      const m = item.exec(raw);
      const value = m === null ? MISS : read(m[0]);
      if (value === MISS) return MISS;
      if (items.length > 0 && !ascending(items[items.length - 1], value)) return MISS;
      items.push(value);
    }
    return Object.freeze(items);
  });
}

// biome-ignore lint/suspicious/noExplicitAny: the field codecs of a record have unrelated types.
type Fields = Readonly<Record<string, Codec<any>>>;
type Values<F extends Fields> = { readonly [K in keyof F]: F[K] extends Codec<infer V> ? V : never };

function objectOf<F extends Fields>(json: Json, fields: F, extra: number): Values<F> {
  const keys = Object.keys(fields);
  if (json.kind !== "object" || json.entries.size !== keys.length + extra) unfit("not an object with these keys");
  const out: Record<string, unknown> = {};
  for (const key of keys) {
    const value = json.entries.get(key);
    if (value === undefined) unfit(`no ${key}`);
    out[key] = fields[key]?.decode(value);
  }
  return out as Values<F>;
}

function membersOf<F extends Fields>(fields: F, values: Values<F>): string[] {
  return Object.keys(fields).map((key) => `"${key}":${fields[key]?.encode(values[key])}`);
}

/** A record: an object with exactly these keys, written in this order. */
export function record<T, F extends Fields>(
  fields: F,
  get: (value: T) => Values<F>,
  make: (values: Values<F>) => T,
): Codec<T> {
  return {
    decode: (json) => make(objectOf(json, fields, 0)),
    encode: (value) => `{${membersOf(fields, get(value)).join(",")}}`,
    fast: fastRecord(fields, make),
  };
}

/** One regular expression for the whole record, a capture group per field, when every field has a shape. */
function fastRecord<T, F extends Fields>(fields: F, make: (values: Values<F>) => T): Fast<T> {
  const keys = Object.keys(fields);
  const fasts = keys.map((key) => (fields[key] as Codec<unknown>).fast);
  if (fasts.some((f) => f.shape === undefined || f.read === undefined)) {
    return { shape: undefined, read: undefined, readAt: () => MISS };
  }
  const member = (wrap: (shape: string) => string) =>
    `\\{${keys.map((key, i) => `"${key}":${wrap(fasts[i]?.shape ?? "")}`).join(",")}\\}`;
  const shape = member((s) => `(?:${s})`);
  const captured = new RegExp(
    member((s) => `(${s})`),
    "y",
  );
  const fromMatch = (m: RegExpExecArray): T | typeof MISS => {
    const out: Record<string, unknown> = {};
    for (let i = 0; i < keys.length; i++) {
      const value = fasts[i]?.read?.(m[i + 1] ?? "") ?? MISS;
      if (value === MISS) return MISS;
      out[keys[i] as string] = value;
    }
    return make(out as Values<F>);
  };
  return {
    shape,
    read: (raw) => {
      captured.lastIndex = 0;
      const m = captured.exec(raw);
      return m === null || captured.lastIndex !== raw.length ? MISS : fromMatch(m);
    },
    readAt: (text, cursor) => {
      captured.lastIndex = cursor.at;
      const m = captured.exec(text);
      if (m === null) return MISS;
      cursor.at = captured.lastIndex;
      return fromMatch(m);
    },
  };
}

/** A Map whose values hold their own key: an array of them, no key twice; written ascending by key. */
export function keyed<K, V>(of: Codec<V>, key: (value: V) => K): Codec<ReadonlyMap<K, V>> {
  return {
    decode: (json) => {
      if (json.kind !== "array") unfit("not an array");
      const entries = json.items.map((item): [K, V] => {
        const value = of.decode(item);
        return [key(value), value];
      });
      if (!entries.every((e, i) => i === 0 || compare(entries[i - 1]?.[0], e[0]) < 0)) {
        entries.sort((a, b) => compare(a[0], b[0]));
        if (entries.some((e, i) => i > 0 && compare(entries[i - 1]?.[0], e[0]) === 0)) unfit("key twice");
      }
      return new Map(entries);
    },
    encode: (value) => `[${[...value.values()].map(of.encode).join(",")}]`,
    fast: {
      shape: undefined,
      read: undefined,
      // The writer emits ascending keys, so that is the only canonical form, and it rules out repeats.
      readAt: (text, cursor) => {
        if (!literal(text, cursor, "[")) return MISS;
        const map = new Map<K, V>();
        let last: K | undefined;
        while (text[cursor.at] !== "]") {
          if (map.size > 0 && !literal(text, cursor, ",")) return MISS;
          const value = of.fast.readAt(text, cursor);
          if (value === MISS) return MISS;
          const k = key(value);
          if (last !== undefined && !ascending(last, k)) return MISS;
          last = k;
          map.set(k, value);
        }
        cursor.at++;
        return map;
      },
    },
  };
}

export type Encoded = { readonly ok: true; readonly text: string } | { readonly ok: false; readonly error: "full" };

/** The whole store file: `{"version":N, ...states}` on one line. */
export interface StoreCodec<W> {
  /** The world, or undefined when the text is not a store of this program. */
  readonly decode: (text: string) => W | undefined;
  readonly encode: (world: W) => Encoded;
}

/**
 * `laws` are the program's laws of the form "in every state, p": a store that breaks one was not
 * written by these rules, so it is refused like any other malformed store.
 */
export function store<W, F extends Fields>(
  version: bigint,
  fields: F,
  get: (world: W) => Values<F>,
  make: (values: Values<F>) => W,
  laws: ReadonlyArray<(world: W) => boolean>,
  speculate: boolean,
): StoreCodec<W> {
  const keys = Object.keys(fields);
  const fast = (text: string): W | undefined => {
    const cursor = { at: 0 };
    if (!literal(text, cursor, `{"version":${version}`)) return undefined;
    const out: Record<string, unknown> = {};
    for (const key of keys) {
      if (!literal(text, cursor, `,"${key}":`)) return undefined;
      const value = (fields[key] as Codec<unknown>).fast.readAt(text, cursor);
      if (value === MISS) return undefined;
      out[key] = value;
    }
    if (text.slice(cursor.at) !== "}\n") return undefined;
    const world = make(out as Values<F>);
    return laws.every((law) => law(world)) ? world : undefined;
  };
  return {
    decode: (text) => {
      // Strategy store.canonical-fast-path: this program wrote the store, so its exact bytes are read first.
      const canonical = speculate ? fast(text) : undefined;
      if (canonical !== undefined) return canonical;
      const json = readJson(text);
      if (!json.ok) return undefined;
      try {
        const found = json.value.kind === "object" ? json.value.entries.get("version") : undefined;
        if (found?.kind !== "number" || found.integer !== version) unfit("version");
        const world = make(objectOf(json.value, fields, 1));
        return laws.every((law) => law(world)) ? world : undefined;
      } catch (e) {
        if (e instanceof Unfit) return undefined;
        throw e;
      }
    },
    encode: (world) => {
      try {
        return { ok: true, text: `{${[`"version":${version}`, ...membersOf(fields, get(world))].join(",")}}\n` };
      } catch (e) {
        if (e instanceof Full) return { ok: false, error: "full" };
        throw e;
      }
    },
  };
}
