// Projected by hand from "every cli rule -> a line of --help" (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import { COMMANDS } from "./commands.ts";

/** Derived once from the command table; the bytes are fixed by contract section 2. */
export const HELP: string = [
  "usage: todo <command> [args]",
  "",
  "commands:",
  ...COMMANDS.map((c) => ["  ", c.name, ...c.params.map((p) => ` ${p.usage}`)].join("")),
  "",
].join("\n");
