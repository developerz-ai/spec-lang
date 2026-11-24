// The spec-lang runtime of the TypeScript printer: a program as data, and an app's manifest. This is the whole
// interface between generated code and a library's runtime (`src/lib/<name>/`): a library reads these, never
// the generated modules by name.

import { Blocked } from "./failure.ts";
import type { Field, Type } from "./types.ts";
import type { Handler, Machine, Note, ParamInfo, RuleInfo, Trace } from "./world.ts";

/** A state, with how to set it on a live world. */
export interface StateInfo<W> extends Field {
  readonly set: (w: W, v: unknown) => void;
}

/** A `fn` as data. */
export interface FnInfo<W> {
  readonly id: string;
  readonly name: string;
  readonly why: string | undefined;
  readonly params: readonly ParamInfo[];
  readonly result: Type;
  readonly call: (w: W, args: readonly unknown[]) => unknown;
}

/** A `law` as data: which parameter is the trace, the rules it traces and those that may build its start. */
export interface LawInfo<W> {
  readonly id: string;
  readonly params: readonly ParamInfo[];
  readonly trace: number;
  readonly traced: readonly RuleInfo<W>[];
  readonly start: readonly RuleInfo<W>[];
  readonly body: (w: W, args: readonly unknown[]) => boolean;
}

/** A `foreign` declaration: a call a library's runtime performs. */
export interface ExternInfo {
  /** The declaration, `module.name`. */
  readonly id: string;
  readonly lang: string;
  readonly library: string;
  readonly symbol: string;
  readonly params: readonly ParamInfo[];
  readonly result: Type;
}

/** The whole program as data: its world, rules, functions, laws and foreign calls. */
export interface Program<W extends Machine, V> {
  /** A world at its initial values, with these effect handlers. */
  readonly world: (handlers: Readonly<Record<string, Handler>>) => W;
  readonly states: readonly StateInfo<W>[];
  /** Every state's value now. */
  readonly value: (w: W) => V;
  readonly rules: readonly RuleInfo<W>[];
  readonly fns: readonly FnInfo<W>[];
  readonly laws: readonly LawInfo<W>[];
  readonly externs: readonly ExternInfo[];
  /** Every effect handler a world holds, by name. */
  readonly handlers: readonly string[];
}

/** A declaration an app parameter names. */
export type Named<W> = { readonly kind: "rule"; readonly rule: RuleInfo<W> } | { readonly kind: "fn"; readonly fn: FnInfo<W> };

/** An `app`: its notes and parameters, as written; what its libraries' runtimes do with them is theirs. */
export interface App<W> {
  readonly name: string;
  readonly qualified: string;
  readonly why: string | undefined;
  readonly target: string | undefined;
  /** The library whose app shape it has. */
  readonly shape: string | undefined;
  readonly notes: readonly Note[];
  /** `rules: [...]`, then the shape's parameters in the shape's order. */
  readonly params: readonly { readonly name: string; readonly decls: readonly Named<W>[] }[];
  /** Every rule the named declarations reach. */
  readonly rules: readonly RuleInfo<W>[];
}

/** The laws a world alone must meet: each law whose one parameter is its trace, on a trace of no steps. */
export function lawful<W extends Machine, V>(program: Program<W, V>, w: W, still: (v: V) => Trace<V>): boolean {
  return program.laws
    .filter((l) => l.params.length === 1)
    .every((l) => l.body(w, [still(program.value(w))]));
}

type Implementation = (args: readonly unknown[], decl: ExternInfo) => unknown;

/** The foreign calls a library's runtime performs, by `library#symbol`. None are bound in the checks. */
const bound = new Map<string, Implementation>();

/** A library's runtime offers its foreign calls. */
export function provide(library: string, calls: Readonly<Record<string, Implementation>>): void {
  for (const [symbol, f] of Object.entries(calls)) bound.set(`${library}#${symbol}`, f);
}

/** A `foreign` body: the library's call when its runtime is here, else the check is blocked. */
export function foreign(decl: ExternInfo, args: readonly unknown[]): unknown {
  const f = bound.get(`${decl.library}#${decl.symbol}`);
  if (f === undefined) {
    const written = `foreign ${decl.lang} "${decl.library}#${decl.symbol}"`;
    throw new Blocked("B-FOREIGN", `reached ${decl.id}, which is ${written}: only a library's runtime runs it`);
  }
  return f(args, decl);
}

/** What a library's handler may read of the program: its foreign declarations and functions. */
export interface ProgramView {
  readonly externs: readonly ExternInfo[];
  readonly fns: readonly FnInfo<never>[];
}

/** A library's runtime, `src/lib/<name>/index.ts`: what it adds to an app. */
export interface Library {
  readonly name: string;
  /** The real effect handlers it implements, by handler name. */
  readonly handlers?: Readonly<Record<string, (program: ProgramView) => Handler>>;
  /** The foreign calls it performs, by symbol. */
  readonly foreign?: Readonly<Record<string, Implementation>>;
  /** Runs the app, when this library gives the app its shape or its commands. */
  readonly main?: <W extends Machine, V>(app: App<W>, program: Program<W, V>, libraries: readonly Library[]) => void;
}

/** Every handler the program holds: a library's real one, or one that keeps nothing for a handler without operations. */
export function realHandlers(
  program: ProgramView & { readonly handlers: readonly string[] },
  libraries: readonly Library[],
): Record<string, Handler> {
  const out: Record<string, Handler> = {};
  for (const name of program.handlers) {
    const make = libraries.find((l) => l.handlers?.[name] !== undefined)?.handlers?.[name];
    out[name] = make?.(program) ?? { save: () => null, restore: () => {} };
  }
  return out;
}

/** Starts an app: binds its libraries' foreign calls, then hands it to the library that runs it. */
export function start<W extends Machine, V>(app: App<W>, program: Program<W, V>, libraries: readonly Library[]): void {
  for (const l of libraries) if (l.foreign !== undefined) provide(l.name, l.foreign);
  const named = app.shape ?? app.notes.find((n) => libraries.some((l) => l.name === n.library && l.main))?.library;
  const driver = libraries.find((l) => l.name === named && l.main !== undefined);
  if (driver?.main === undefined) throw new Error(`app ${app.name}: no library here runs it`);
  driver.main(app, program, libraries);
}
