// Values drawn from declared types, claims included: the engine's algorithm, step for step, so the same seed
// gives the same values.

import * as rt from "../../src/runtime/index.ts";
import type { Rng } from "./rng.ts";

/** Tries per value before a claim counts as unmet. */
const TRIES = 400;

/** No value of a type was found. */
export class Exhausted extends Error {}

/** Values already in a world, by declared type: the keys of each `Map[K, V]` state, the elements of each `Set[K]`. */
export class Pools {
  private readonly pools = new Map<string, { readonly type: rt.Type; items: rt.OrdSet<unknown> }>();

  static of(states: readonly rt.Field[], worlds: Iterable<unknown>): Pools {
    const p = new Pools();
    for (const w of worlds) {
      for (const s of states) {
        const v = (w as Record<string, unknown>)[s.prop];
        if (s.type instanceof rt.MapType && v instanceof rt.OrdMap) p.add(s.type.keys, v.keyList);
        else if (s.type instanceof rt.SetType && v instanceof rt.OrdSet) p.add(s.type.of, v.items);
      }
    }
    return p;
  }

  private add(type: rt.Type, values: readonly unknown[]): void {
    let pool = this.pools.get(type.key);
    if (pool === undefined) {
      pool = { type, items: rt.OrdSet.empty(type) };
      this.pools.set(type.key, pool);
    }
    for (const v of values) pool.items = pool.items.insert(v);
  }

  get(type: rt.Type): readonly unknown[] | undefined {
    const items = this.pools.get(type.key)?.items.items;
    return items === undefined || items.length === 0 ? undefined : items;
  }
}

const empty = new Pools();

/** A value of `type`: half the time (when a pool holds that type) one of the pool, else a fresh one; kept when it conforms. */
export function value(rng: Rng, type: rt.Type, pools: Pools = empty): unknown {
  for (let i = 0; i < TRIES; i++) {
    const pool = pools.get(type);
    const v = pool !== undefined && rng.chance(1n, 2n) ? rng.pick(pool) : base(rng, type, pools);
    if (type.conform(v) === undefined) return v;
  }
  throw new Exhausted(`no value of \`${type.key}\` found in ${TRIES} tries`);
}

function base(rng: Rng, type: rt.Type, pools: Pools): unknown {
  if (type instanceof rt.UnitType) return null;
  if (type instanceof rt.BoolType) return rng.chance(1n, 2n);
  if (type instanceof rt.RangeType) {
    const hi = type.inclusive ? type.hi : type.hi - 1n;
    if (rng.chance(1n, 5n)) return rng.chance(1n, 2n) ? type.lo : hi;
    return rng.between(type.lo, hi);
  }
  if (type instanceof rt.IntType) return int(rng);
  if (type instanceof rt.TextType) return text(rng);
  if (type instanceof rt.OptionType) return rng.chance(1n, 3n) ? rt.none : rt.some(value(rng, type.of, pools));
  if (type instanceof rt.ListType) {
    const n = rng.below(4n);
    const xs: unknown[] = [];
    for (let i = 0n; i < n; i++) xs.push(value(rng, type.of, pools));
    return xs;
  }
  if (type instanceof rt.SetType) {
    const n = rng.below(4n);
    let s = rt.OrdSet.empty(type.of);
    for (let i = 0n; i < n; i++) s = s.insert(value(rng, type.of, pools));
    return s;
  }
  if (type instanceof rt.MapType) {
    const n = rng.below(4n);
    let m = rt.OrdMap.empty<unknown, unknown>(type.keys);
    for (let i = 0n; i < n; i++) {
      const k = value(rng, type.keys, pools);
      m = m.set(k, value(rng, type.values, pools));
    }
    return m;
  }
  if (type instanceof rt.TupleType) return type.items.map((t) => value(rng, t, pools));
  if (type instanceof rt.WhereType) return value(rng, type.base, pools);
  if (type instanceof rt.AliasType) return value(rng, type.of, pools);
  if (type instanceof rt.RecordType) {
    const out: Record<string, unknown> = {};
    for (const f of type.fields) out[f.prop] = value(rng, f.type, pools);
    return out;
  }
  if (type instanceof rt.SumType && !type.builtin) {
    const v = type.variants[Number(rng.below(BigInt(type.variants.length)))];
    const out: Record<string, unknown> = { kind: v?.name ?? "" };
    for (const f of v?.fields ?? []) out[f.prop] = value(rng, f.type, pools);
    return out;
  }
  throw new Exhausted(`no generator for \`${type.key}\``);
}

/** Boundaries a machine word breaks at. */
const EDGES: readonly bigint[] = [
  0n,
  1n,
  -1n,
  2_147_483_647n,
  2_147_483_648n,
  9_007_199_254_740_992n,
  9_007_199_254_740_993n,
  9_223_372_036_854_775_807n,
  9_223_372_036_854_775_808n,
  -9_223_372_036_854_775_808n,
  18_446_744_073_709_551_616n,
];

/** Small numbers most of the time, then boundaries a machine word breaks at. */
function int(rng: Rng): bigint {
  const band = rng.below(100n);
  if (band < 35n) return rng.between(0n, 10n);
  if (band < 55n) return rng.between(11n, 100n);
  if (band < 70n) return rng.between(-10n, -1n);
  if (band < 80n) return rng.pick(EDGES) ?? 0n;
  if (band < 90n) return rng.between(101n, 1n << 62n);
  return -rng.between(11n, 1n << 62n);
}

const WORD = "abcdefghijklmnopqrstuvwxyz0123456789-";
const MIX = [
  "a",
  "Z",
  " ",
  "\t",
  "\n",
  "é",
  "ß",
  "中",
  "😀",
  "　",
  "\u0085",
  " ",
  "​",
  "﻿",
  "\u007f",
  "\u0001",
  '"',
  "\\",
  "{",
  "}",
  "#",
  "-",
];

/** Shell-safe words, printable ASCII, a mix with Unicode space, controls and astral characters, or empty. */
function text(rng: Rng): string {
  const draw = (n: bigint, one: () => string): string => {
    let s = "";
    for (let i = 0n; i < n; i++) s += one();
    return s;
  };
  switch (rng.below(4n)) {
    case 0n:
      return draw(1n + rng.below(8n), () => rng.pick([...WORD]) ?? "a");
    case 1n:
      return draw(rng.below(13n), () => String.fromCharCode(32 + Number(rng.below(95n))));
    case 2n:
      return draw(rng.below(9n), () => rng.pick(MIX) ?? "a");
    default:
      return "";
  }
}
