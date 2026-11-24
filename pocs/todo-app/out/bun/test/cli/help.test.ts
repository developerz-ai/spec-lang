// Projected by hand from "every cli rule -> a line of --help" (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import { HELP } from "../../src/cli/help.ts";

test("help is derived from the command table and matches the contract byte for byte", () => {
  expect(HELP).toBe(
    [
      "usage: todo <command> [args]",
      "",
      "commands:",
      "  add <title> [--priority low|normal|high] [--tags <tag>]...",
      "  list [--all] [--tag <tag>]",
      "  done <no>",
      "  reopen <no>",
      "  rename <no> <title>",
      "  remove <no>",
      "  clear",
      "  stats",
      "",
    ].join("\n"),
  );
});
