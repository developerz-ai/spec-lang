// Projected by hand from the argument rules of the cli vocabulary (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import { describe, expect, test } from "bun:test";
import { type Parsed, parse } from "../../src/cli/parse.ts";

/** A readable summary: `help:stdout`, `error: ...`, or the command and its values. */
function show(p: Parsed): string {
  if (p.kind === "help") return `help:${p.stream}`;
  if (p.kind === "usage") return `error: ${p.message}`;
  const values = p.values.map((v) => (v instanceof Set ? `{${[...v].join(",")}}` : String(v)));
  return [p.command.name, ...values].join(" ");
}

describe("parse", () => {
  test.each([
    [[], "help:stderr"],
    [["--help"], "help:stdout"],
    [["-h"], "help:stdout"],
    [["add", "-h"], "help:stdout"],
    [["frobnicate", "--help"], "help:stdout"],
    [["add", "x", "--priority", "--help"], "help:stdout"],
    [["add", "--", "--help"], "add --help Normal {}"],
    [["add", "--priority", "--", "-h"], "error: --priority: not a Priority"],
    [["--"], 'error: unknown command "--"'],
    [["Add", "x"], 'error: unknown command "Add"'],
    [["add"], "error: missing <title>"],
    [["rename", "1"], "error: missing <title>"],
    [["add", "buy", "milk"], 'error: unexpected argument "milk"'],
    [["clear", "now"], 'error: unexpected argument "now"'],
    [["done", "0"], "error: <no>: not a TaskNo"],
    [["done", "-1"], "error: <no>: not a TaskNo"],
    [["done", "+1"], "error: <no>: not a TaskNo"],
    [["done", "1.5"], "error: <no>: not a TaskNo"],
    [["done", ""], "error: <no>: not a TaskNo"],
    [["done", " 1"], "error: <no>: not a TaskNo"],
    [["done", "١"], "error: <no>: not a TaskNo"],
    [["done", "007"], "done 7"],
    [["done", "99999999999999999999999999999999"], "done 99999999999999999999999999999999"],
    [["add", "x", "--force"], "error: unknown flag --force"],
    [["add", "x", "--priority=high"], "error: unknown flag --priority=high"],
    [["add", "x", "---priority", "high"], "error: unknown flag ---priority"],
    [["add", "--title", "x"], "error: unknown flag --title"],
    [["done", "1", "--all"], "error: unknown flag --all"],
    [["add", "x", "--priority"], "error: --priority needs a value"],
    [["add", "x", "--priority", "High"], "error: --priority: not a Priority"],
    [["add", "x", "--priority", "--tags"], "error: --priority: not a Priority"],
    [["add", "x", "--priority", "high", "--priority", "low"], "error: --priority given twice"],
    [["add", "x", "--priority", "high", "--priority"], "error: --priority needs a value"],
    [["add", "x", "--tags", "Home"], "error: --tags: not a Tag"],
    [["add", "x", "--tags", ""], "error: --tags: not a Tag"],
    [["add", "x", "--tags", "-x"], "error: --tags: not a Tag"],
    [["add", "x", "--tags", "--all"], "error: --tags: not a Tag"],
    [["list", "--tag", "--all"], "error: --tag: not a Tag"],
    [["add", "a\u2028b"], "error: <title>: not a Title"],
    [["add", "a\u2029b"], "error: <title>: not a Title"],
    [["add", "a\u0085b"], "error: <title>: not a Title"],
    [["list", "--all", "--all"], "error: --all given twice"],
    [["list", "--tag", "a", "--tag", "b"], "error: --tag given twice"],
    [["done", "abc", "--force"], "error: <no>: not a TaskNo"],
    [["done", "--force", "abc"], "error: unknown flag --force"],
    [["add", "x", "y", "--priority", "nope"], 'error: unexpected argument "y"'],
    [["add", "--priority", "nope"], "error: --priority: not a Priority"],
    [["rename", "x"], "error: <no>: not a TaskNo"],
    [["add", "--priority", "high", "ship it", "--tags", "work"], "add ship it High {work}"],
    [["add", "-x"], "add -x Normal {}"],
    [["add", "-"], "add - Normal {}"],
    [["add", "--", "--all", "--"], 'error: unexpected argument "--"'],
    [["add", "x", "--tags", "b", "--tags", "a", "--tags", "b"], "add x Normal {b,a}"],
    [["list"], "list false undefined"],
    [["list", "--tag", "home", "--all"], "list true home"],
    [["rename", "3", "urgent, really"], "rename 3 urgent, really"],
  ])("%j -> %s", (argv, want) => {
    expect(show(parse(argv))).toBe(want);
  });
});
