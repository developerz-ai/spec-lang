// The `cli` library's runtime for TypeScript (libs/cli/real/ts): the store of an app's world between runs.
// The file format, derived from the states' declared types alone:
//   {"version":1, <each state that is not a collection>, <each collection>}, each group in declaration order,
//   keyed by the state's name. One line of JSON and a newline, no other white space.
//   Int: an integer within 2^53 (a larger one is `full` when written, `unreadable` when read).
//   Text: a string; only `"` and `\` are escaped, and what JSON cannot hold raw.  Bool: true or false.
//   A record: an object of its fields in declared order.  A sum without data: its variant's name.
//   A sum with data: {"<variant>": {<fields>}}.  Option: null, or the value.  List, Set, tuple: an array.
//   Map[K, R] whose record R starts with a field of type K holding each entry's key: the array of its records;
//   any other Map: an array of [key, value] pairs.
// A read accepts any valid JSON with exactly these keys, then checks every value against its declared type.

import * as rt from "../../../runtime/index.ts";
import type { Json } from "./json-reader.ts";

/** A value the store cannot hold: an Int past 2^53. */
export class Full extends Error {}

/** Valid JSON that is not a world of this program. */
export class Unfit extends Error {}

function unfit(why: string): never {
  throw new Unfit(why);
}

/** The largest magnitude of a stored Int: a store constraint, not a language rule. */
export const MAX_INT = 2n ** 53n;

/** The type as stored: an alias or a claim stores as its base. */
function base(t: rt.Type): rt.Type {
  if (t instanceof rt.AliasType) return base(t.of);
  if (t instanceof rt.WhereType) return base(t.base);
  return t;
}

// biome-ignore lint/suspicious/noControlCharactersInRegex: JSON requires these escaped.
const ESCAPE = /["\\\x00-\x1f]/g;

function quote(s: string): string {
  return `"${s.replace(ESCAPE, (c) => (c === '"' || c === "\\" ? `\\${c}` : `\\u${c.charCodeAt(0).toString(16).padStart(4, "0")}`))}"`;
}

/** A map stored as the list of its records: its values are records whose first field holds the key. */
function keyedRecords(t: rt.MapType<unknown, unknown>): rt.RecordType<unknown> | undefined {
  const v = base(t.values);
  if (!(v instanceof rt.RecordType)) return undefined;
  const first = v.fields[0];
  return first !== undefined && first.type.key === t.keys.key ? v : undefined;
}

export function encode(t: rt.Type, v: unknown): string {
  const b = base(t);
  if (b instanceof rt.IntType) {
    const n = v as bigint;
    if (n > MAX_INT || n < -MAX_INT) throw new Full();
    return String(n);
  }
  if (b instanceof rt.TextType) return quote(v as string);
  if (b instanceof rt.BoolType) return String(v);
  if (b instanceof rt.UnitType) return "null";
  if (b instanceof rt.OptionType) {
    const o = v as rt.Option<unknown>;
    return o.kind === "Some" ? encode(b.of, o.value) : "null";
  }
  if (b instanceof rt.ListType) return `[${(v as readonly unknown[]).map((x) => encode(b.of, x)).join(",")}]`;
  if (b instanceof rt.SetType) return `[${(v as rt.OrdSet<unknown>).items.map((x) => encode(b.of, x)).join(",")}]`;
  if (b instanceof rt.TupleType) {
    return `[${(v as readonly unknown[]).map((x, i) => encode(b.items[i] ?? rt.anyType, x)).join(",")}]`;
  }
  if (b instanceof rt.MapType) {
    const m = v as rt.OrdMap<unknown, unknown>;
    const records = keyedRecords(b);
    const first = records?.fields[0];
    if (records !== undefined && first !== undefined) {
      const keyed = m.keyList.every((k, i) => rt.equal(k, (m.valueList[i] as Record<string, unknown>)[first.prop]));
      if (keyed) return `[${m.valueList.map((x) => encode(records, x)).join(",")}]`;
    }
    return `[${m.keyList.map((k, i) => `[${encode(b.keys, k)},${encode(b.values, m.valueList[i])}]`).join(",")}]`;
  }
  if (b instanceof rt.RecordType) return fields(b.fields, v as Record<string, unknown>);
  if (b instanceof rt.SumType) {
    const s = v as { readonly kind: string } & Record<string, unknown>;
    const variant = b.variantOf(s);
    if (variant === undefined || variant.fields.length === 0) return quote(s.kind);
    return `{${quote(s.kind)}:${fields(variant.fields, s)}}`;
  }
  throw new Error(`a state of type ${t.key} has no stored form`);
}

function fields(fs: readonly rt.Field[], v: Record<string, unknown>): string {
  return `{${fs.map((f) => `${quote(f.name)}:${encode(f.type, v[f.prop])}`).join(",")}}`;
}

export function decode(t: rt.Type, json: Json): unknown {
  const b = base(t);
  if (b instanceof rt.IntType) {
    if (json.kind !== "number" || json.integer === undefined) unfit("not an integer");
    if (json.integer > MAX_INT || json.integer < -MAX_INT) unfit("integer out of range");
    return json.integer;
  }
  if (b instanceof rt.TextType) return json.kind === "string" ? json.value : unfit("not a string");
  if (b instanceof rt.BoolType) return json.kind === "bool" ? json.value : unfit("not a boolean");
  if (b instanceof rt.UnitType) return json.kind === "null" ? null : unfit("not null");
  if (b instanceof rt.OptionType) return json.kind === "null" ? rt.none : rt.some(decode(b.of, json));
  if (json.kind === "array" && b instanceof rt.ListType) return json.items.map((x) => decode(b.of, x));
  if (json.kind === "array" && b instanceof rt.SetType) {
    const items = json.items.map((x) => decode(b.of, x));
    const set = rt.OrdSet.of(b.of, items);
    if (set.items.length !== items.length) unfit("an element twice");
    return set;
  }
  if (json.kind === "array" && b instanceof rt.TupleType) {
    if (json.items.length !== b.items.length) unfit("a tuple of another size");
    return json.items.map((x, i) => decode(b.items[i] ?? rt.anyType, x));
  }
  if (json.kind === "array" && b instanceof rt.MapType) return map(b, json.items);
  if (json.kind === "object" && b instanceof rt.RecordType) return record(b.fields, json.entries, {});
  if (b instanceof rt.SumType) {
    if (json.kind === "string") {
      const variant = b.variants[b.rank(json.value)];
      if (variant === undefined || variant.fields.length > 0) unfit("not a variant");
      return { kind: json.value };
    }
    if (json.kind === "object" && json.entries.size === 1) {
      const [[kind, inner]] = [...json.entries] as [[string, Json]];
      const variant = b.variants[b.rank(kind)];
      if (variant === undefined || variant.fields.length === 0 || inner.kind !== "object") unfit("not a variant");
      return record(variant.fields, inner.entries, { kind });
    }
  }
  return unfit(`not a ${t.key}`);
}

function record(fs: readonly rt.Field[], entries: ReadonlyMap<string, Json>, out: Record<string, unknown>): unknown {
  if (entries.size !== fs.length) unfit("keys other than the fields");
  for (const f of fs) {
    const j = entries.get(f.name);
    if (j === undefined) unfit(`no ${f.name}`);
    out[f.prop] = decode(f.type, j);
  }
  return out;
}

function map(t: rt.MapType<unknown, unknown>, items: readonly Json[]): rt.OrdMap<unknown, unknown> {
  const records = keyedRecords(t);
  const first = records?.fields[0];
  let m = rt.OrdMap.empty<unknown, unknown>(t.keys);
  for (const item of items) {
    let k: unknown;
    let v: unknown;
    if (item.kind === "object" && records !== undefined && first !== undefined) {
      v = decode(records, item);
      k = (v as Record<string, unknown>)[first.prop];
    } else if (item.kind === "array" && item.items.length === 2) {
      const [kj, vj] = item.items as [Json, Json];
      k = decode(t.keys, kj);
      v = decode(t.values, vj);
    } else {
      unfit("not an entry");
    }
    if (m.has(k)) unfit("a key twice");
    m = m.set(k, v);
  }
  return m;
}

/** A state as stored: its name, unique among the app's states. */
export interface Stored<W> {
  readonly key: string;
  readonly state: rt.StateInfo<W>;
}

/** The states in file order: every one that is not a collection, then the collections. */
export function layout<W>(states: readonly rt.StateInfo<W>[]): readonly Stored<W>[] {
  const leaf = (s: rt.StateInfo<W>) => s.name.slice(s.name.lastIndexOf(".") + 1);
  const key = (s: rt.StateInfo<W>) => (states.filter((o) => leaf(o) === leaf(s)).length > 1 ? s.name : leaf(s));
  const collection = (s: rt.StateInfo<W>) => {
    const b = base(s.type);
    return b instanceof rt.ListType || b instanceof rt.SetType || b instanceof rt.MapType;
  };
  return [...states.filter((s) => !collection(s)), ...states.filter(collection)].map((s) => ({ key: key(s), state: s }));
}

/** The world's bytes. Throws `Full` when a value does not fit. */
export function encodeWorld<W, V>(layout: readonly Stored<W>[], world: V): string {
  const parts = layout.map((s) => `${quote(s.key)}:${encode(s.state.type, (world as Record<string, unknown>)[s.state.prop])}`);
  return `{"version":1,${parts.join(",")}}\n`;
}

/** Sets every state of `w` from the stored JSON; each value is checked against its declared type. */
export function decodeWorld<W>(layout: readonly Stored<W>[], json: Json, w: W): void {
  if (json.kind !== "object" || json.entries.size !== layout.length + 1) unfit("not the store's keys");
  const version = json.entries.get("version");
  if (version?.kind !== "number" || version.integer !== 1n) unfit("not version 1");
  for (const s of layout) {
    const j = json.entries.get(s.key);
    if (j === undefined) unfit(`no ${s.key}`);
    const v = decode(s.state.type, j);
    if (s.state.type.conform(v) !== undefined) unfit(`${s.key} outside its type`);
    s.state.set(w, v);
  }
}
