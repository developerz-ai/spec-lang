// The checks' runner: every `@cases` row, test, property and law of the program, run as `spec-lang test` runs
// them, each with one verdict: pass, fail or blocked.

import * as rt from "../../src/runtime/index.ts";
import { Exhausted, Pools, value } from "./generate.ts";
import { Rng, seedFor } from "./rng.ts";

export type Status = "pass" | "fail" | "blocked";

/** A test check did not hold (`F-TEST`). */
export class CheckFailed extends Error {}

/** `assert cond`. */
export function assert(holds: boolean): void {
  if (!holds) throw new CheckFailed("an assert is false");
}

/** `subject -> value`. */
export function expect(subject: () => unknown, want: () => unknown): void {
  const got = rt.outcome(subject);
  const wanted = want();
  if (!got.ok) throw new CheckFailed(`the step was refused: ${got.failure.failure}`);
  if (!rt.equal(got.value, wanted)) throw new CheckFailed("the step gave another value");
}

/** `subject -> fail Name(data)`: the name, and the data when some is given. */
export function expectFail(subject: () => unknown, name: string, data: () => readonly unknown[] = () => []): void {
  const got = rt.outcome(subject);
  const wanted = data();
  if (got.ok) throw new CheckFailed(`the step gave a value, not fail ${name}`);
  const same = got.failure.failure === name && (wanted.length === 0 || rt.equal(got.failure.data, wanted));
  if (!same) throw new CheckFailed(`the step failed with ${got.failure.failure}, not ${name}`);
}

/** The program as the runner reads it: its world, its tests, its laws. */
export interface Program<W extends rt.Machine, V> {
  /** A machine at the initial world: every state's initial value, checked; fresh handlers. */
  readonly fresh: () => W;
  readonly states: readonly State<W>[];
  /** Every state's value now. */
  readonly value: (w: W) => V;
  readonly tests: Readonly<Record<string, Test<W>>>;
  readonly laws: readonly rt.LawInfo<W>[];
}

export type State<W> = rt.StateInfo<W>;

export type Given<W> = { readonly state: State<W>; readonly value: (w: W) => unknown } | { readonly test: string };

export interface Test<W> {
  readonly id: string;
  readonly params: readonly rt.ParamInfo[];
  readonly given: readonly Given<W>[];
  readonly body: (w: W, args: readonly unknown[]) => void;
}

export interface Check<W extends rt.Machine, V> {
  readonly name: string;
  readonly run: (r: Runner<W, V>) => Status;
}

/** Counts and the base seed, `spec-lang test`'s defaults. */
export interface Config {
  readonly seed: bigint;
  readonly runs: number;
  readonly traces: number;
  readonly steps: number;
  readonly prefix: number;
}

export const DEFAULTS: Config = { seed: 0n, runs: 100, traces: 200, steps: 40, prefix: 10 };

/** How a run of a test ended: its world, or why not. */
type Run<W> = { readonly status: "pass"; readonly world: W } | { readonly status: "fail" | "blocked"; readonly code: string };

/** With `SPEC_CHECKS_WHY` set, why a check did not pass goes to standard error. */
function why(e: unknown): void {
  if (process.env.SPEC_CHECKS_WHY) console.error(e);
}

function ended(e: unknown): { readonly status: "fail" | "blocked"; readonly code: string } {
  why(e);
  if (e instanceof rt.Blocked) return { status: "blocked", code: e.code };
  if (e instanceof rt.Stop) return { status: "fail", code: e.code };
  return { status: "fail", code: "F-TEST" };
}

/** A verdict from a halt outside a test's steps: only a foreign body blocks. */
function foreignOrFail(e: unknown): Status {
  why(e);
  return e instanceof rt.Blocked && e.code === "B-FOREIGN" ? "blocked" : "fail";
}

export class Runner<W extends rt.Machine, V> {
  /** End worlds of tests named by `@given`: every state's value, never the handlers'. */
  private readonly ends = new Map<string, Run<V>>();
  private readonly running: string[] = [];

  constructor(
    readonly program: Program<W, V>,
    readonly config: Config = DEFAULTS,
  ) {}

  /** A `@cases` row of a `fn`: a fresh world, one call, then the expected value. */
  fnCase(call: (w: W) => unknown, want: (w: W) => unknown): Status {
    try {
      const w = this.program.fresh();
      const got = rt.outcome(() => call(w));
      if (!got.ok) return "fail";
      return rt.equal(got.value, want(w)) ? "pass" : "fail";
    } catch (e) {
      return blockedOnly(e);
    }
  }

  /** A `@cases` row expecting a refusal: compared by name only. */
  fnCaseFails(call: (w: W) => unknown, name: string): Status {
    try {
      const w = this.program.fresh();
      const got = rt.outcome(() => call(w));
      return !got.ok && got.failure.failure === name ? "pass" : "fail";
    } catch (e) {
      return blockedOnly(e);
    }
  }

  /** A `@cases` row of a test with parameters: its `@given` world, then the test with the row's values. */
  testCase(id: string, args: (w: W) => readonly unknown[]): Status {
    const start = this.given(id);
    if (start.status !== "pass") return start.status;
    let values: readonly unknown[];
    try {
      values = args(start.world);
    } catch (e) {
      return blockedOnly(e);
    }
    return this.runTest(id, values).status;
  }

  test(id: string): Status {
    return this.endOf(id).status;
  }

  /** `runs` runs with generated arguments; the first that does not pass decides. */
  property(id: string): Status {
    const test = this.test$(id);
    for (let run = 0; run < this.config.runs; run++) {
      const rng = new Rng(seedFor(this.config.seed, id, run));
      let args: unknown[];
      try {
        this.program.fresh();
        args = test.params.map((p) => value(rng, p.type));
      } catch (e) {
        return foreignOrFail(e);
      }
      const r = new Runner(this.program, this.config).runTest(id, args);
      if (r.status !== "pass") return r.status;
    }
    return "pass";
  }

  law(id: string): Status {
    const law = this.program.laws.find((l) => l.id === id);
    if (law === undefined) return "fail";
    for (let i = 0; i < this.config.traces; i++) {
      try {
        const rng = new Rng(seedFor(this.config.seed, id, i));
        const start = i % 2 === 1 ? this.lawfulWorld(rng) : undefined;
        const trace = this.trace(rng, law, start);
        const others = this.others(rng, law, trace.states);
        if (!this.holds(law, trace, others)) return "fail";
      } catch (e) {
        return foreignOrFail(e);
      }
    }
    return "pass";
  }

  private test$(id: string): Test<W> {
    const t = this.program.tests[id];
    if (t === undefined) throw new Error(`no test ${id}`);
    return t;
  }

  private endOf(id: string): Run<V> {
    const known = this.ends.get(id);
    if (known !== undefined) return known;
    const r = this.runTest(id, []);
    const end: Run<V> = r.status === "pass" ? { status: "pass", world: this.program.value(r.world) } : r;
    this.ends.set(id, end);
    return end;
  }

  /** A fresh world, then each `@given` item in order. */
  private given(id: string): Run<W> {
    let w: W;
    try {
      w = this.program.fresh();
    } catch (e) {
      return ended(e);
    }
    for (const g of this.test$(id).given) {
      if ("state" in g) {
        try {
          g.state.set(w, rt.check(g.state.type, g.value(w), "this `@given` value"));
        } catch (e) {
          return { status: "fail", code: ended(e).code };
        }
        continue;
      }
      if (this.running.includes(g.test) || g.test === id) return { status: "fail", code: "E-TYPE-MISMATCH" };
      this.running.push(id);
      const parent = this.endOf(g.test);
      this.running.pop();
      if (parent.status === "pass") {
        for (const s of this.program.states) s.set(w, (parent.world as Record<string, unknown>)[s.prop]);
        continue;
      }
      return { status: "blocked", code: parent.status === "blocked" && parent.code === "B-FOREIGN" ? "B-FOREIGN" : "F-BLOCKED" };
    }
    return { status: "pass", world: w };
  }

  private runTest(id: string, args: readonly unknown[]): Run<W> {
    const start = this.given(id);
    if (start.status !== "pass") return start;
    try {
      this.test$(id).body(start.world, args);
    } catch (e) {
      return ended(e);
    }
    return { status: "pass", world: start.world };
  }

  /** A world drawn from the states' declared types on which every law holds at the start. */
  private lawfulWorld(rng: Rng): V | undefined {
    for (let i = 0; i < 200; i++) {
      const w = this.program.fresh();
      for (const s of this.program.states) s.set(w, value(rng, s.type));
      const world = this.program.value(w);
      const still = rt.still(world);
      let lawful = true;
      for (const law of this.program.laws) {
        const others = this.others(rng, law, [world]);
        if (!this.holds(law, still, others, true)) {
          lawful = false;
          break;
        }
      }
      if (lawful) return world;
    }
    return undefined;
  }

  /** Every parameter but the trace, drawn with pools of the trace's worlds. */
  private others(rng: Rng, law: rt.LawInfo<W>, worlds: readonly V[]): unknown[] {
    const pools = Pools.of(this.program.states, worlds);
    return law.params.flatMap((p, i) => (i === law.trace ? [] : [value(rng, p.type, pools)]));
  }

  /** The law on a trace, in a fresh machine. `quiet`: a law that does not give true is simply not lawful. */
  private holds(law: rt.LawInfo<W>, trace: rt.Trace<V>, others: readonly unknown[], quiet = false): boolean {
    const rest = [...others];
    const args = law.params.map((_, i) => (i === law.trace ? trace : rest.shift()));
    if (!quiet) return law.body(this.program.fresh(), args);
    try {
      return law.body(this.program.fresh(), args);
    } catch {
      return false;
    }
  }

  private event(rng: Rng, w: W, rules: readonly rt.RuleInfo<W>[]): rt.Event {
    const rule = rng.pick(rules);
    if (rule === undefined) throw new Exhausted("a trace with no rules");
    const pools = Pools.of(this.program.states, [this.program.value(w)]);
    return { rule: rule as rt.RuleInfo<never>, args: rule.params.map((p) => value(rng, p.type, pools)) };
  }

  private call(w: W, e: rt.Event): rt.Reply {
    return rt.reply(() => (e.rule as unknown as rt.RuleInfo<W>).call(w, e.args));
  }

  private trace(rng: Rng, law: rt.LawInfo<W>, start: V | undefined): rt.Trace<V> {
    const w = this.program.fresh();
    let prefix = 0n;
    if (start === undefined) prefix = rng.below(BigInt(this.config.prefix) + 1n);
    else for (const s of this.program.states) s.set(w, (start as Record<string, unknown>)[s.prop]);
    for (let i = 0n; i < prefix; i++) this.call(w, this.event(rng, w, law.start));
    const states: V[] = [this.program.value(w)];
    const steps: rt.Step<V>[] = [];
    for (let i = 0; i < this.config.steps; i++) {
      const event = this.event(rng, w, law.traced);
      const reply = this.call(w, event);
      const after = this.program.value(w);
      steps.push({ event, reply, before: states[states.length - 1] as V, after });
      states.push(after);
    }
    return { steps, states };
  }
}

/** A check that stopped: blocked by a hole or a foreign body, otherwise failed. */
function blockedOnly(e: unknown): Status {
  why(e);
  return e instanceof rt.Blocked ? "blocked" : "fail";
}

/** Runs every check in order and prints one JSON line each: what `bin/test` does. */
export function report<W extends rt.Machine, V>(program: Program<W, V>, checks: readonly Check<W, V>[]): void {
  const runner = new Runner(program);
  for (const c of checks) {
    let status: Status;
    try {
      status = c.run(runner);
    } catch (e) {
      status = blockedOnly(e);
    }
    process.stdout.write(`${JSON.stringify({ check: c.name, status })}\n`);
  }
}
