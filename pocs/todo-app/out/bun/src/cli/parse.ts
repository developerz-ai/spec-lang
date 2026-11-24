// Projected by hand from the argument rules of the cli vocabulary (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import { COMMAND_BY_NAME, type Command } from "./commands.ts";
import { MISSING } from "./params.ts";

export type Parsed =
  | { readonly kind: "help"; readonly stream: "stdout" | "stderr" }
  | { readonly kind: "usage"; readonly message: string }
  | { readonly kind: "run"; readonly command: Command; readonly values: readonly unknown[] };

const END_OF_FLAGS = "--";

function usage(message: string): Parsed {
  return { kind: "usage", message };
}

/** Contract section 2, "Parsing, in this order". The first problem from the left is the one reported. */
export function parse(argv: readonly string[]): Parsed {
  if (argv.length === 0) return { kind: "help", stream: "stderr" };
  for (const a of argv) {
    if (a === END_OF_FLAGS) break;
    if (a === "--help" || a === "-h") return { kind: "help", stream: "stdout" };
  }
  const [name = "", ...rest] = argv;
  const command = COMMAND_BY_NAME.get(name);
  if (command === undefined) return usage(`unknown command "${name}"`);
  return parseArgs(command, rest);
}

function parseArgs(command: Command, args: readonly string[]): Parsed {
  const { params } = command;
  const values: unknown[] = params.map((p) => p.fallback);
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
        // A switch takes no value.
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
        values[at] = p.shape === "repeat" ? new Set([...(values[at] as ReadonlySet<unknown>), value]) : value;
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
