// The `cli` library's runtime for TypeScript (libs/cli/real/ts): commands from the manifest. Each rule with
// trigger `cli("name")` is a command; its parameters become arguments by the derivation of the vocabulary card:
//   a parameter without a default: a positional argument `<name>`
//   `Bool` with a default: a switch `--name`
//   `Option[T]` or any other type with a default: `--name <value>`, at most once
//   `Set[T]` with a default: `--name <value>`, repeatable; repeats collapse
//   a sum without data: its variant names in lower case
// A value is parsed by its base type, then checked against its declared type: outside it is a usage error.

import * as rt from "../../runtime/index.ts";

/** How one argument becomes a value; `undefined` means "not a <name>". */
interface ArgType {
  /** The declared type's name, as errors say it. */
  readonly name: string;
  /** What `--help` shows after a flag of this type. */
  readonly hint: string;
  readonly parse: (text: string) => unknown;
}

type Shape = "positional" | "switch" | "once" | "repeat";

export interface Param {
  readonly name: string;
  readonly shape: Shape;
  /** Absent for a switch. */
  readonly type: ArgType | undefined;
  /** For a repeated flag: the set's element type. */
  readonly set: rt.Type | undefined;
  /** For an option flag: a parsed value is `Some(value)`. */
  readonly some: boolean;
  /** This parameter's part of the command's `--help` line. */
  readonly usage: string;
}

export interface Command<W> {
  readonly name: string;
  readonly rule: rt.RuleInfo<W>;
  readonly params: readonly Param[];
}

const DIGITS = /^-?[0-9]+$/;

/** The type's name: an alias's, a declared type's, or the base's. */
function typeName(t: rt.Type): string {
  if (t instanceof rt.AliasType || t instanceof rt.RecordType || t instanceof rt.SumType) return t.name;
  if (t instanceof rt.WhereType) return typeName(t.base);
  if (t instanceof rt.IntType) return "Int";
  if (t instanceof rt.TextType) return "Text";
  if (t instanceof rt.BoolType) return "Bool";
  return t.key;
}

function base(t: rt.Type): rt.Type {
  if (t instanceof rt.AliasType) return base(t.of);
  if (t instanceof rt.WhereType) return base(t.base);
  return t;
}

/** The argument form of a declared type: parsed by its base, then checked whole. */
function argType(t: rt.Type): ArgType {
  const name = typeName(t);
  const b = base(t);
  const checked = (v: unknown): unknown => (v !== undefined && t.conform(v) === undefined ? v : undefined);
  if (b instanceof rt.SumType && b.variants.every((v) => v.fields.length === 0)) {
    const byName = new Map(b.variants.map((v) => [v.name.toLowerCase(), v.name]));
    return {
      name,
      hint: [...byName.keys()].join("|"),
      parse: (text) => {
        const kind = byName.get(text);
        return checked(kind === undefined ? undefined : b.make(kind));
      },
    };
  }
  const hint = `<${name.toLowerCase()}>`;
  if (b instanceof rt.IntType) return { name, hint, parse: (text) => checked(DIGITS.test(text) ? BigInt(text) : undefined) };
  if (b instanceof rt.BoolType) {
    return { name, hint, parse: (text) => checked(text === "true" ? true : text === "false" ? false : undefined) };
  }
  if (b instanceof rt.TextType) return { name, hint, parse: (text) => checked(text.isWellFormed() ? text : undefined) };
  return { name, hint, parse: () => undefined };
}

function param(p: rt.ParamInfo): Param {
  const b = base(p.type);
  if (!p.hasDefault) {
    return { name: p.name, shape: "positional", type: argType(p.type), set: undefined, some: false, usage: `<${p.name}>` };
  }
  if (b instanceof rt.BoolType) {
    return { name: p.name, shape: "switch", type: undefined, set: undefined, some: false, usage: `[--${p.name}]` };
  }
  if (b instanceof rt.SetType) {
    const type = argType(b.of);
    return { name: p.name, shape: "repeat", type, set: b.of, some: false, usage: `[--${p.name} ${type.hint}]...` };
  }
  const some = b instanceof rt.OptionType;
  const type = argType(b instanceof rt.OptionType ? b.of : p.type);
  return { name: p.name, shape: "once", type, set: undefined, some, usage: `[--${p.name} ${type.hint}]` };
}

/** Every rule of the app with a `cli` trigger, in the app's order. */
export function commands<W>(rules: readonly rt.RuleInfo<W>[]): readonly Command<W>[] {
  const out: Command<W>[] = [];
  for (const rule of rules) {
    for (const t of rule.triggers) {
      const arg = t.args[0]?.value;
      if (t.library === "cli" && t.name === "cli" && arg?.kind === "text") {
        out.push({ name: arg.value, rule, params: rule.params.map(param) });
      }
    }
  }
  return out;
}

export function helpText<W>(bin: string, commands: readonly Command<W>[]): string {
  return [
    `usage: ${bin} <command> [args]`,
    "",
    "commands:",
    ...commands.map((c) => ["  ", c.name, ...c.params.map((p) => ` ${p.usage}`)].join("")),
    "",
  ].join("\n");
}

export type Parsed<W> =
  | { readonly kind: "help"; readonly stream: "stdout" | "stderr" }
  | { readonly kind: "usage"; readonly message: string }
  | { readonly kind: "run"; readonly command: Command<W>; readonly values: readonly unknown[] };

const END_OF_FLAGS = "--";
const MISSING: unique symbol = Symbol("missing");

function usage(message: string): { readonly kind: "usage"; readonly message: string } {
  return { kind: "usage", message };
}

/** The parsing order of the cli vocabulary. The first problem from the left is the one reported. */
export function parse<W>(commands: readonly Command<W>[], argv: readonly string[]): Parsed<W> {
  if (argv.length === 0) return { kind: "help", stream: "stderr" };
  for (const a of argv) {
    if (a === END_OF_FLAGS) break;
    if (a === "--help" || a === "-h") return { kind: "help", stream: "stdout" };
  }
  const [name = "", ...rest] = argv;
  const command = commands.find((c) => c.name === name);
  if (command === undefined) return usage(`unknown command "${name}"`);
  return parseArgs(command, rest);
}

function parseArgs<W>(command: Command<W>, args: readonly string[]): Parsed<W> {
  const { params } = command;
  // `undefined` lets the rule's own default apply.
  const values: unknown[] = params.map((p) => (p.shape === "positional" ? MISSING : undefined));
  const seen: boolean[] = params.map(() => false);
  const positionals = params.flatMap((p, i) => (p.shape === "positional" ? [i] : []));
  let filled = 0;
  let flagsEnded = false;

  for (let i = 0; i < args.length; i++) {
    const arg = args[i] as string;
    if (!flagsEnded && arg === END_OF_FLAGS) {
      flagsEnded = true;
      continue;
    }
    if (!flagsEnded && arg.startsWith("--")) {
      const at = params.findIndex((p) => p.shape !== "positional" && `--${p.name}` === arg);
      const p = params[at];
      if (p === undefined) return usage(`unknown flag ${arg}`);
      if (p.type === undefined) {
        if (seen[at]) return usage(`${arg} given twice`);
        values[at] = true;
      } else {
        // A value flag takes the next argument, whatever it is.
        const text = args[i + 1];
        if (text === undefined) return usage(`${arg} needs a value`);
        i++;
        if (p.shape === "once" && seen[at]) return usage(`${arg} given twice`);
        const value = p.type.parse(text);
        if (value === undefined) return usage(`${arg}: not a ${p.type.name}`);
        if (p.shape === "repeat" && p.set !== undefined) {
          const before = values[at] instanceof rt.OrdSet ? (values[at] as rt.OrdSet<unknown>) : rt.OrdSet.empty(p.set);
          values[at] = before.insert(value);
        } else {
          values[at] = p.some ? rt.some(value) : value;
        }
      }
      seen[at] = true;
      continue;
    }
    const at = positionals[filled];
    const p = at === undefined ? undefined : params[at];
    if (at === undefined || p?.type === undefined) return usage(`unexpected argument "${arg}"`);
    const value = p.type.parse(arg);
    if (value === undefined) return usage(`<${p.name}>: not a ${p.type.name}`);
    values[at] = value;
    filled++;
  }

  const missing = params.find((p, i) => values[i] === MISSING && p.shape === "positional");
  if (missing !== undefined) return usage(`missing <${missing.name}>`);
  return { kind: "run", command, values };
}
