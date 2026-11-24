// The spec-lang runtime of the TypeScript printer: types as values.
// A `Type` knows how to order, show and check (`conform`) a value of its type. The printer builds one per
// declared type and composes them where a value needs one (a Set's elements, a sort key, a checked parameter);
// a library's runtime reads their structure (a store codec, an argument parser, a foreign call).

import { OrdMap, OrdSet, type Range } from "./collections.ts";
import { Failure, Stop, stop } from "./failure.ts";
import type { Int } from "./int.ts";
import { compareText, quote } from "./text.ts";
import type { Option } from "./values.ts";

export abstract class Type<T = unknown> {
  /** Equal keys mean the same declared type: what a generator's pools are looked up by. */
  abstract readonly key: string;
  /** The value order: negative, zero or positive. */
  abstract compare(a: T, b: T): number;
  /** As spec-lang source, which `"{x}"` prints for anything but Text. */
  abstract show(v: T): string;
  /** Why `v` is not of this type, or undefined when it is: ranges and claims included. */
  conform(_v: T): string | undefined {
    return undefined;
  }
}

function sign(n: number): number {
  return n < 0 ? -1 : n > 0 ? 1 : 0;
}

export class UnitType extends Type<null> {
  readonly key = "()";
  compare(): number {
    return 0;
  }
  show(): string {
    return "()";
  }
}

export class BoolType extends Type<boolean> {
  readonly key = "Bool";
  compare(a: boolean, b: boolean): number {
    return Number(a) - Number(b);
  }
  show(v: boolean): string {
    return v ? "true" : "false";
  }
}

export class IntType extends Type<Int> {
  readonly key: string = "Int";
  compare(a: Int, b: Int): number {
    return a < b ? -1 : a > b ? 1 : 0;
  }
  show(v: Int): string {
    return v.toString();
  }
}

export class TextType extends Type<string> {
  readonly key = "Text";
  compare(a: string, b: string): number {
    return sign(compareText(a, b));
  }
  show(v: string): string {
    return quote(v);
  }
}

/** `lo..=hi` or `lo..<hi` as a type: an Int inside. */
export class RangeType extends IntType {
  override readonly key: string;
  constructor(
    readonly lo: Int,
    readonly hi: Int,
    readonly inclusive: boolean,
  ) {
    super();
    this.key = `Range(${lo},${hi},${inclusive})`;
  }
  override conform(v: Int): string | undefined {
    const inside = v >= this.lo && (this.inclusive ? v <= this.hi : v < this.hi);
    return inside ? undefined : `${v} is not in ${this.lo}${this.inclusive ? "..=" : "..<"}${this.hi}`;
  }
}

/** `T where claim`: the base first, then the claim. */
export class WhereType<T> extends Type<T> {
  readonly key: string;
  constructor(
    readonly base: Type<T>,
    readonly claim: (self: T) => boolean,
    readonly text: string,
    id: number,
  ) {
    super();
    this.key = `Where(${base.key},${id})`;
  }
  compare(a: T, b: T): number {
    return this.base.compare(a, b);
  }
  show(v: T): string {
    return this.base.show(v);
  }
  override conform(v: T): string | undefined {
    const base = this.base.conform(v);
    if (base !== undefined) return base;
    try {
      return this.claim(v) ? undefined : `\`${this.text}\` is false for ${this.show(v)}`;
    } catch (e) {
      if (e instanceof Failure) return `the claim failed with ${e.failure}`;
      throw e;
    }
  }
}

export class OptionType<T> extends Type<Option<T>> {
  readonly key: string;
  constructor(readonly of: Type<T>) {
    super();
    this.key = `Option[${of.key}]`;
  }
  compare(a: Option<T>, b: Option<T>): number {
    if (a.kind === "Some" && b.kind === "Some") return this.of.compare(a.value, b.value);
    return (a.kind === "Some" ? 0 : 1) - (b.kind === "Some" ? 0 : 1);
  }
  show(v: Option<T>): string {
    return v.kind === "Some" ? `Some(${this.of.show(v.value)})` : "none";
  }
  override conform(v: Option<T>): string | undefined {
    return v.kind === "Some" ? this.of.conform(v.value) : undefined;
  }
}

function compareItems<T>(t: (i: number) => Type<T>, a: readonly T[], b: readonly T[]): number {
  const n = Math.min(a.length, b.length);
  for (let i = 0; i < n; i++) {
    const d = t(i).compare(a[i] as T, b[i] as T);
    if (d !== 0) return d;
  }
  return sign(a.length - b.length);
}

function conformItems<T>(t: (i: number) => Type<T>, xs: Iterable<T>): string | undefined {
  let i = 0;
  for (const x of xs) {
    const why = t(i++).conform(x);
    if (why !== undefined) return why;
  }
  return undefined;
}

export class ListType<T> extends Type<readonly T[]> {
  readonly key: string;
  constructor(readonly of: Type<T>) {
    super();
    this.key = `List[${of.key}]`;
  }
  compare(a: readonly T[], b: readonly T[]): number {
    return compareItems(() => this.of, a, b);
  }
  show(v: readonly T[]): string {
    return `[${v.map((x) => this.of.show(x)).join(", ")}]`;
  }
  override conform(v: readonly T[]): string | undefined {
    return conformItems(() => this.of, v);
  }
}

export class SetType<T> extends Type<OrdSet<T>> {
  readonly key: string;
  constructor(readonly of: Type<T>) {
    super();
    this.key = `Set[${of.key}]`;
  }
  compare(a: OrdSet<T>, b: OrdSet<T>): number {
    return compareItems(() => this.of, a.items, b.items);
  }
  show(v: OrdSet<T>): string {
    return `Set(${v.items.map((x) => this.of.show(x)).join(", ")})`;
  }
  override conform(v: OrdSet<T>): string | undefined {
    return conformItems(() => this.of, v.items);
  }
}

export class MapType<K, V> extends Type<OrdMap<K, V>> {
  readonly key: string;
  constructor(
    readonly keys: Type<K>,
    readonly values: Type<V>,
  ) {
    super();
    this.key = `Map[${keys.key},${values.key}]`;
  }
  compare(a: OrdMap<K, V>, b: OrdMap<K, V>): number {
    const n = Math.min(a.keyList.length, b.keyList.length);
    for (let i = 0; i < n; i++) {
      const d = this.keys.compare(a.keyList[i] as K, b.keyList[i] as K);
      if (d !== 0) return d;
      const e = this.values.compare(a.valueList[i] as V, b.valueList[i] as V);
      if (e !== 0) return e;
    }
    return sign(a.keyList.length - b.keyList.length);
  }
  show(v: OrdMap<K, V>): string {
    const items = v.keyList.map((k, i) => `${this.keys.show(k)} -> ${this.values.show(v.valueList[i] as V)}`);
    return `Map(${items.join(", ")})`;
  }
  override conform(v: OrdMap<K, V>): string | undefined {
    return conformItems(() => this.keys, v.keyList) ?? conformItems(() => this.values, v.valueList);
  }
}

export class TupleType extends Type<readonly unknown[]> {
  readonly key: string;
  constructor(readonly items: readonly Type[]) {
    super();
    this.key = `(${items.map((t) => t.key).join(",")})`;
  }
  private at(i: number): Type {
    return this.items[i] ?? anyType;
  }
  compare(a: readonly unknown[], b: readonly unknown[]): number {
    return compareItems((i) => this.at(i), a, b);
  }
  show(v: readonly unknown[]): string {
    return `(${v.map((x, i) => this.at(i).show(x)).join(", ")})`;
  }
  override conform(v: readonly unknown[]): string | undefined {
    return conformItems((i) => this.at(i), v);
  }
}

/** One field of a record or a variant: its name in spec-lang, its property in TypeScript, its declared type. */
export interface Field {
  readonly name: string;
  readonly prop: string;
  readonly type: Type;
  /** Evaluated when the field is not given. */
  readonly default?: () => unknown;
}

export function field(name: string, prop: string, type: Type, fallback?: () => unknown): Field {
  return fallback === undefined ? { name, prop, type } : { name, prop, type, default: fallback };
}

type Fields = Readonly<Record<string, unknown>>;

function showFields(fields: readonly Field[], v: Fields): string {
  return fields
    .map((f, i) => (i === 0 ? f.type.show(v[f.prop]) : `${f.name}: ${f.type.show(v[f.prop])}`))
    .join(", ");
}

function compareFields(fields: readonly Field[], a: Fields, b: Fields): number {
  for (const f of fields) {
    const d = f.type.compare(a[f.prop], b[f.prop]);
    if (d !== 0) return d;
  }
  return 0;
}

function conformFields(fields: readonly Field[], v: Fields): string | undefined {
  for (const f of fields) {
    const why = f.type.conform(v[f.prop]);
    if (why !== undefined) return `field \`${f.name}\`: ${why}`;
  }
  return undefined;
}

/** Fills absent fields with their defaults, then checks each, in order: a checked constructor. */
function build(fields: readonly Field[], args: readonly unknown[], into: Record<string, unknown>): void {
  fields.forEach((f, i) => {
    let v = args[i];
    if (v === undefined) {
      if (f.default === undefined) stop("E-RUNTIME", `missing field \`${f.name}\``);
      v = f.default();
    }
    const why = f.type.conform(v);
    if (why !== undefined) throw new ClaimBroken(`field \`${f.name}\`: ${why}`);
    into[f.prop] = v;
  });
}

/** A value broke its declared type (`E-CLAIM`). */
export class ClaimBroken extends Stop {
  constructor(message: string) {
    super("E-CLAIM", message);
  }
}

/** A declared type with a name: a record, a sum or an alias. Its structure is read on first use. */
abstract class Named<T> extends Type<T> {
  readonly key: string;
  constructor(
    /** `module.Name`. */
    readonly qualified: string,
  ) {
    super();
    this.key = `Named(${qualified})`;
  }
  get name(): string {
    return this.qualified.slice(this.qualified.lastIndexOf(".") + 1);
  }
}

export class RecordType<T> extends Named<T> {
  private cached: readonly Field[] | undefined;
  constructor(
    qualified: string,
    private readonly declare: () => readonly Field[],
  ) {
    super(qualified);
  }
  get fields(): readonly Field[] {
    this.cached ??= this.declare();
    return this.cached;
  }
  /** `Name(args)`: `undefined` takes the field's default; each field is checked. */
  make(args: readonly unknown[]): T {
    const out: Record<string, unknown> = {};
    build(this.fields, args, out);
    return out as T;
  }
  /** `r.with(f: v)`: a copy with fields changed, each checked. */
  with(r: T, changes: Readonly<Partial<T>>): T {
    const out: Record<string, unknown> = { ...(r as Fields) };
    for (const [prop, v] of Object.entries(changes)) {
      const f = this.fields.find((x) => x.prop === prop);
      const why = f?.type.conform(v);
      if (f !== undefined && why !== undefined) throw new ClaimBroken(`field \`${f.name}\`: ${why}`);
      out[prop] = v;
    }
    return out as T;
  }
  compare(a: T, b: T): number {
    return compareFields(this.fields, a as Fields, b as Fields);
  }
  show(v: T): string {
    return `${this.name}(${showFields(this.fields, v as Fields)})`;
  }
  override conform(v: T): string | undefined {
    return conformFields(this.fields, v as Fields);
  }
}

export interface Variant {
  readonly name: string;
  readonly fields: readonly Field[];
}

export function variant(name: string, fields: readonly Field[] = []): Variant {
  return { name, fields };
}

/** A sum: values are `{ kind: "<variant>", ...fields }`. */
export class SumType<T extends { readonly kind: string }> extends Named<T> {
  private cached: readonly Variant[] | undefined;
  private index: ReadonlyMap<string, number> | undefined;
  constructor(
    qualified: string,
    private readonly declare: () => readonly Variant[],
    /** `Option` and `Result`: declared by the language; their payload is checked where they are used. */
    readonly builtin = false,
  ) {
    super(qualified);
  }
  get variants(): readonly Variant[] {
    this.cached ??= this.declare();
    return this.cached;
  }
  /** The variant's position in the declaration: the value order of a sum. */
  rank(kind: string): number {
    this.index ??= new Map(this.variants.map((v, i) => [v.name, i]));
    return this.index.get(kind) ?? -1;
  }
  variantOf(v: T): Variant | undefined {
    return this.variants[this.rank(v.kind)];
  }
  make(kind: string, args: readonly unknown[] = []): T {
    const out: Record<string, unknown> = { kind };
    build(this.variants[this.rank(kind)]?.fields ?? [], args, out);
    return out as T;
  }
  compare(a: T, b: T): number {
    const d = this.rank(a.kind) - this.rank(b.kind);
    if (d !== 0) return sign(d);
    return compareFields(this.variantOf(a)?.fields ?? [], a as Fields, b as Fields);
  }
  show(v: T): string {
    const fields = this.variantOf(v)?.fields ?? [];
    return fields.length === 0 ? v.kind : `${v.kind}(${showFields(fields, v as unknown as Fields)})`;
  }
  override conform(v: T): string | undefined {
    if (this.builtin) return undefined;
    const variant = this.variantOf(v);
    if (variant === undefined) return `${v.kind} is not a declared variant`;
    return conformFields(variant.fields, v as unknown as Fields);
  }
}

/** `type Title = Text where is_title(self)`: a value of the base that conforms. */
export class AliasType<T> extends Named<T> {
  private cached: Type<T> | undefined;
  constructor(
    qualified: string,
    private readonly declare: () => Type<T>,
  ) {
    super(qualified);
  }
  get of(): Type<T> {
    this.cached ??= this.declare();
    return this.cached;
  }
  compare(a: T, b: T): number {
    return this.of.compare(a, b);
  }
  show(v: T): string {
    return this.of.show(v);
  }
  override conform(v: T): string | undefined {
    return this.of.conform(v);
  }
}

/** A value whose type the program does not state where it is shown: a failure's data, a reply's value. */
class AnyType extends Type {
  readonly key = "?";
  compare(a: unknown, b: unknown): number {
    return compareAny(a, b);
  }
  show(v: unknown): string {
    return showAny(v);
  }
}

export const anyType: Type = new AnyType();

/** Order without a type: only for values the program cannot sort by type (a reply's payload). */
export function compareAny(a: unknown, b: unknown): number {
  if (typeof a === "bigint" && typeof b === "bigint") return a < b ? -1 : a > b ? 1 : 0;
  if (typeof a === "string" && typeof b === "string") return sign(compareText(a, b));
  if (typeof a === "boolean" && typeof b === "boolean") return Number(a) - Number(b);
  if (Array.isArray(a) && Array.isArray(b)) return compareItems(() => anyType, a, b);
  if (a instanceof OrdSet && b instanceof OrdSet) return compareItems(() => anyType, a.items, b.items);
  if (typeof a === "object" && a !== null && typeof b === "object" && b !== null) {
    return compareItems(() => anyType, Object.values(a), Object.values(b));
  }
  return 0;
}

/** As spec-lang source without a type: exact for Int, Text, Bool, `()`, lists and options. */
export function showAny(v: unknown): string {
  if (v === null) return "()";
  if (typeof v === "bigint" || typeof v === "boolean") return String(v);
  if (typeof v === "string") return quote(v);
  if (Array.isArray(v)) return `[${v.map(showAny).join(", ")}]`;
  if (v instanceof OrdSet) return `Set(${v.items.map(showAny).join(", ")})`;
  if (v instanceof OrdMap) return `Map(${v.keyList.map((k, i) => `${showAny(k)} -> ${showAny(v.valueList[i])}`).join(", ")})`;
  if (typeof v === "object" && "kind" in v) {
    const { kind, ...rest } = v as { kind: string };
    const fields = Object.values(rest);
    return fields.length === 0 ? kind : `${kind}(${fields.map(showAny).join(", ")})`;
  }
  return String(v);
}

export class RangeValueType extends Type<Range> {
  readonly key = "Range";
  compare(a: Range, b: Range): number {
    return compareAny([a.lo, a.hi, a.inclusive], [b.lo, b.hi, b.inclusive]);
  }
  show(v: Range): string {
    return `${v.lo}${v.inclusive ? "..=" : "..<"}${v.hi}`;
  }
}

/** A function value: never compared or shown by a program that checks. */
export class FuncType extends Type<unknown> {
  readonly key = "Func";
  compare(): number {
    return 0;
  }
  show(): string {
    return "<function>";
  }
}

export const unit = new UnitType();
export const bool = new BoolType();
export const int = new IntType();
export const text = new TextType();
export const rangeValue = new RangeValueType();
export const func = new FuncType();

/** Composite types are made once per element type, so equal types are one object. */
const made = new Map<string, Type>();

function once<T extends Type>(key: string, make: () => T): T {
  let t = made.get(key);
  if (t === undefined) {
    t = make();
    made.set(key, t);
  }
  return t as T;
}

export function list<T>(of: Type<T>): ListType<T> {
  return once(`List[${of.key}]`, () => new ListType(of));
}

export function set<T>(of: Type<T>): SetType<T> {
  return once(`Set[${of.key}]`, () => new SetType(of));
}

export function map<K, V>(keys: Type<K>, values: Type<V>): MapType<K, V> {
  return once(`Map[${keys.key},${values.key}]`, () => new MapType(keys, values));
}

export function option<T>(of: Type<T>): OptionType<T> {
  return once(`Option[${of.key}]`, () => new OptionType(of));
}

export function tuple(...items: readonly Type[]): TupleType {
  return once(`(${items.map((t) => t.key).join(",")})`, () => new TupleType(items));
}

export function inRange(lo: Int, hi: Int, inclusive: boolean): RangeType {
  return once(`Range(${lo},${hi},${inclusive})`, () => new RangeType(lo, hi, inclusive));
}

/** `base where claim`; `id` tells two claims apart. */
export function where<T>(base: Type<T>, claim: (self: T) => boolean, text: string, id: number): WhereType<T> {
  return once(`Where(${base.key},${id})`, () => new WhereType(base, claim, text, id));
}

export function record<T>(qualified: string, fields: () => readonly Field[]): RecordType<T> {
  return new RecordType(qualified, fields);
}

export function oneOf<T extends { readonly kind: string }>(qualified: string, variants: () => readonly Variant[]): SumType<T> {
  return new SumType(qualified, variants);
}

export function alias<T>(qualified: string, of: () => Type<T>): AliasType<T> {
  return new AliasType(qualified, of);
}

/** The value, when it conforms to `type`; else evaluation stops (`E-CLAIM`). */
export function check<T>(type: Type<T>, v: T, what = "the value"): T {
  const why = type.conform(v);
  if (why !== undefined) throw new ClaimBroken(`${what}: ${why}`);
  return v;
}

/** A field of a sum value by name; a variant without it stops (`E-TYPE-MISMATCH`). */
export function fieldOf<T>(v: { readonly kind: string }, prop: string): T {
  if (!(prop in v)) stop("E-TYPE-MISMATCH", `${v.kind} has no \`${prop}\``);
  return (v as unknown as Record<string, T>)[prop] as T;
}

/** A `match` none of whose arms matches. */
export function noMatch(): never {
  return stop("E-RUNTIME", "no arm matches");
}

/** A constant: computed once, on first use. */
export function lazy<T>(make: () => T): () => T {
  let done = false;
  let value: T | undefined;
  return () => {
    if (!done) {
      value = make();
      done = true;
    }
    return value as T;
  };
}

/** The order of `type` as a comparator. */
export function order<T>(type: Type<T>): (a: T, b: T) => number {
  return (a, b) => type.compare(a, b);
}
