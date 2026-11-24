// The spec-lang runtime of the TypeScript printer: the world a rule changes, and what a trace records of it.

import { type Failure, outcome } from "./failure.ts";
import { anyType, compareAny, type Field, ListType, TupleType, Type } from "./types.ts";

/** Everything a rule may change: the states, and what each effect handler keeps. */
export interface Machine {
  /** A copy of everything, cheap: states hold immutable values. */
  save(): unknown;
  restore(saved: unknown): void;
}

/** What an effect handler keeps, so a refused rule takes back what it sent. */
export interface Handler {
  save(): unknown;
  restore(saved: unknown): void;
}

/**
 * A rule's body as one step: on any failure or stop the world and every handler go back to what they were
 * before, for a rule called inside a rule as well; otherwise its writes join the caller's.
 */
export function atomic<T>(w: Machine, body: () => T): T {
  const saved = w.save();
  try {
    return body();
  } catch (e) {
    w.restore(saved);
    throw e;
  }
}

/** One parameter of a rule, as a trace generator and a library's runtime read it. */
export interface ParamInfo {
  readonly name: string;
  /** Its declared type: ranges and claims included. */
  readonly type: Type;
  readonly hasDefault: boolean;
  /** The default, when it reads nothing: no earlier parameter, no state. */
  readonly default?: () => unknown;
}

/** A library's note or trigger: `cli("add")`, `tick(60Hz)`. */
export interface Note {
  readonly library: string;
  readonly name: string;
  readonly args: readonly { readonly param: string; readonly value: NoteValue }[];
}

export type NoteValue =
  | { readonly kind: "int"; readonly value: bigint }
  | { readonly kind: "text"; readonly value: string }
  | { readonly kind: "quantity"; readonly value: bigint; readonly unit: string }
  | { readonly kind: "variant"; readonly name: string; readonly args: Note["args"] };

/** A rule as data: what a trace calls, what a library's runtime offers its user. */
export interface RuleInfo<W = never> {
  /** `module.name`. */
  readonly id: string;
  readonly name: string;
  /** The last part of the module's path, as an event shows it. */
  readonly module: string;
  readonly why: string | undefined;
  readonly params: readonly ParamInfo[];
  readonly result: Type;
  readonly triggers: readonly Note[];
  /** The rule's write set is empty: it reads the world and changes nothing in it. */
  readonly reads: boolean;
  /** Calls the rule; `undefined` in `args` takes the parameter's default. */
  readonly call: (w: W, args: readonly unknown[]) => unknown;
}

/** A call of a rule, as a trace records it. */
export interface Event {
  readonly rule: RuleInfo<never>;
  readonly args: readonly unknown[];
}

/** A refusal as a value: a reply's `Err`. */
export interface FailureValue {
  readonly name: string;
  readonly data: readonly unknown[];
}

export type Reply =
  | { readonly kind: "Ok"; readonly value: unknown }
  | { readonly kind: "Err"; readonly error: FailureValue };

export interface Step<V> {
  readonly event: Event;
  readonly reply: Reply;
  readonly before: V;
  readonly after: V;
}

export interface Trace<V> {
  readonly steps: readonly Step<V>[];
  /** The start world, then the world after each step. */
  readonly states: readonly V[];
}

/** What a reply's `Ok` holds: its rule's result. */
const okTypes = new WeakMap<Reply, Type>();

/** A rule called from outside: its reply, `Err` for a refusal. A stop is not a reply: it propagates. */
export function reply<W>(rule: RuleInfo<W>, w: W, args: readonly unknown[]): Reply {
  const r = outcome(() => rule.call(w, args));
  if (!r.ok) return { kind: "Err", error: failureValue(r.failure) };
  const value: Reply = { kind: "Ok", value: r.value };
  okTypes.set(value, rule.result);
  return value;
}

/** The types of a failure's data, by the data. */
const dataTypes = new WeakMap<readonly unknown[], readonly Type[]>();

export function failureValue(f: Failure): FailureValue {
  dataTypes.set(f.data, f.types as readonly Type[]);
  return { name: f.failure, data: f.data };
}

function sign(n: number): number {
  return n < 0 ? -1 : n > 0 ? 1 : 0;
}

export class FailureType extends Type<FailureValue> {
  readonly key = "Failure";
  compare(a: FailureValue, b: FailureValue): number {
    const d = compareAny(a.name, b.name);
    return d !== 0 ? d : new TupleType(dataTypes.get(a.data) ?? []).compare(a.data, b.data);
  }
  show(v: FailureValue): string {
    const types = dataTypes.get(v.data) ?? [];
    const shown = v.data.map((x, i) => (types[i] ?? anyType).show(x));
    return shown.length === 0 ? `fail ${v.name}` : `fail ${v.name}(${shown.join(", ")})`;
  }
}

export class EventType extends Type<Event> {
  readonly key = "Event";
  compare(a: Event, b: Event): number {
    if (a.rule.id !== b.rule.id) return a.rule.id < b.rule.id ? -1 : 1;
    for (let i = 0; i < a.args.length; i++) {
      const d = (a.rule.params[i]?.type ?? anyType).compare(a.args[i], b.args[i]);
      if (d !== 0) return d;
    }
    return 0;
  }
  show(v: Event): string {
    const args = v.args.map((x, i) => {
      const p = v.rule.params[i];
      const shown = (p?.type ?? anyType).show(x);
      return i === 0 || p === undefined ? shown : `${p.name}: ${shown}`;
    });
    return `${v.rule.module}.${v.rule.name}(${args.join(", ")})`;
  }
}

export class ReplyType extends Type<Reply> {
  readonly key = "Reply";
  compare(a: Reply, b: Reply): number {
    if (a.kind === "Ok" && b.kind === "Ok") return (okTypes.get(a) ?? anyType).compare(a.value, b.value);
    if (a.kind === "Err" && b.kind === "Err") return failure.compare(a.error, b.error);
    return a.kind === "Ok" ? -1 : 1;
  }
  show(v: Reply): string {
    return v.kind === "Ok" ? `Ok(${(okTypes.get(v) ?? anyType).show(v.value)})` : `Err(${failure.show(v.error)})`;
  }
}

/** Every state at one moment, keyed by property, shown by qualified name. */
export class WorldType<V> extends Type<V> {
  readonly key = "World";
  private cached: readonly Field[] | undefined;
  constructor(private readonly declare: () => readonly Field[]) {
    super();
  }
  get states(): readonly Field[] {
    this.cached ??= this.declare();
    return this.cached;
  }
  compare(a: V, b: V): number {
    for (const s of this.states) {
      const d = s.type.compare((a as Record<string, unknown>)[s.prop], (b as Record<string, unknown>)[s.prop]);
      if (d !== 0) return d;
    }
    return 0;
  }
  show(v: V): string {
    const items = this.states.map((s) => `${s.name}: ${s.type.show((v as Record<string, unknown>)[s.prop])}`);
    return `World(${items.join(", ")})`;
  }
}

export class StepType<V> extends Type<Step<V>> {
  readonly key = "Step";
  constructor(private readonly world: Type<V>) {
    super();
  }
  compare(a: Step<V>, b: Step<V>): number {
    return (
      event.compare(a.event, b.event) ||
      replies.compare(a.reply, b.reply) ||
      this.world.compare(a.before, b.before) ||
      this.world.compare(a.after, b.after)
    );
  }
  show(v: Step<V>): string {
    return `${event.show(v.event)} -> ${replies.show(v.reply)}`;
  }
}

export class TraceType<V> extends Type<Trace<V>> {
  readonly key = "Trace";
  constructor(private readonly world: Type<V>) {
    super();
  }
  compare(a: Trace<V>, b: Trace<V>): number {
    const steps = new StepType(this.world);
    const n = Math.min(a.steps.length, b.steps.length);
    for (let i = 0; i < n; i++) {
      const d = steps.compare(a.steps[i] as Step<V>, b.steps[i] as Step<V>);
      if (d !== 0) return d;
    }
    if (a.steps.length !== b.steps.length) return sign(a.steps.length - b.steps.length);
    return new ListType(this.world).compare(a.states, b.states);
  }
  show(v: Trace<V>): string {
    return `<trace of ${v.steps.length} steps>`;
  }
}

export const failure = new FailureType();
export const event = new EventType();
export const replies = new ReplyType();

/** A trace of no steps over one world: what a law says about a world alone. */
export function still<V>(world: V): Trace<V> {
  return { steps: [], states: [world] };
}

/** `s.event.no`: an argument of a traced call by name; its position per rule. */
export function eventArg(e: Event, name: string, at: Readonly<Record<string, number>>): unknown {
  const i = at[e.rule.id];
  if (i === undefined || i >= e.args.length) throw new TypeError(`the event has no argument \`${name}\``);
  return e.args[i];
}
