// Projected by hand from argument parsing for app SkyhopBun (docs/game-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import type { LevelNo } from "../levels/level.ts";
import { type Command, type CommandName, type Flag, flagsOf } from "./command.ts";

export type Parsed =
  | { readonly kind: "help"; readonly to: "stdout" | "stderr" }
  | { readonly kind: "usage"; readonly message: string }
  | { readonly kind: "command"; readonly command: Command };

const usage = (message: string): Parsed => ({ kind: "usage", message });

const isCommand = (s: string): s is CommandName => Object.hasOwn(flagsOf, s);

/** ASCII digits, value 1 or 2, leading zeros allowed. */
function levelNo(text: string): LevelNo | null {
  const m = /^0*([12])$/.exec(text);
  return m === null ? null : m[1] === "1" ? 1 : 2;
}

/** ASCII digits, any value 0 or more: a BigInt, so a tick of any length is exact. */
const tick = (text: string): bigint | null => (/^[0-9]+$/.test(text) ? BigInt(text) : null);

export function parse(argv: readonly string[]): Parsed {
  const first = argv[0];
  if (first === undefined) return { kind: "help", to: "stderr" };
  if (argv.includes("--help") || argv.includes("-h")) return { kind: "help", to: "stdout" };
  if (!isCommand(first)) return usage(`unknown command "${first}"`);
  const allowed = flagsOf[first];

  let level: LevelNo | undefined;
  let at: bigint | undefined;
  let out: string | undefined;
  const seen = new Set<Flag>();
  for (let i = 1; i < argv.length; i++) {
    const arg = argv[i] as string;
    if (!arg.startsWith("--")) return usage(`unexpected argument "${arg}"`);
    const flag = allowed.find((f) => `--${f}` === arg);
    if (flag === undefined) return usage(`unknown flag ${arg}`);
    const value = argv[++i];
    if (value === undefined) return usage(`${arg} needs a value`);
    if (seen.has(flag)) return usage(`${arg} given twice`);
    seen.add(flag);
    if (flag === "level") {
      const n = levelNo(value);
      if (n === null) return usage("--level: not a LevelNo");
      level = n;
    } else if (flag === "at") {
      const n = tick(value);
      if (n === null) return usage("--at: not a tick");
      at = n;
    } else {
      if (value === "") return usage("--out: not a path");
      out = value;
    }
  }

  const lv = level ?? 1;
  switch (first) {
    case "play":
    case "run":
      return { kind: "command", command: { name: first, level: lv } };
    case "draw":
      return { kind: "command", command: { name: first, level: lv, at: at ?? null } };
    case "shot":
      return { kind: "command", command: { name: first, level: lv, at: at ?? null, out: out ?? null } };
  }
}
