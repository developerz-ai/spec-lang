// Projected by hand from argument parsing for app SkyhopBun (docs/game-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import { type Parsed, parse } from "../../src/cli/parse.ts";

const usage = (message: string): Parsed => ({ kind: "usage", message });

test("no arguments: help on standard error", () => {
  expect(parse([])).toEqual({ kind: "help", to: "stderr" });
});

test.each([
  [["--help"]],
  [["-h"]],
  [["run", "--level", "9", "-h"]],
  [["draw", "--at", "-h"]],
  [["jump", "--help"]],
])("help anywhere: %j", (argv) => {
  expect(parse(argv)).toEqual({ kind: "help", to: "stdout" });
});

test.each<[string[], string]>([
  [["jump"], 'unknown command "jump"'],
  [["--level"], 'unknown command "--level"'],
  [[""], 'unknown command ""'],
  [["run", "--fast"], "unknown flag --fast"],
  [["run", "--level=2"], "unknown flag --level=2"],
  [["run", "--at", "3"], "unknown flag --at"],
  [["play", "--out", "x.png"], "unknown flag --out"],
  [["draw", "--out", "x.png"], "unknown flag --out"],
  [["run", "--"], "unknown flag --"],
  [["run", "--level"], "--level needs a value"],
  [["draw", "--at", "1", "--at", "2"], "--at given twice"],
  [["draw", "--level", "1", "--level"], "--level needs a value"],
  [["draw", "--at", "1", "--at", "x"], "--at given twice"],
  [["run", "--level", "0"], "--level: not a LevelNo"],
  [["run", "--level", "3"], "--level: not a LevelNo"],
  [["run", "--level", "-1"], "--level: not a LevelNo"],
  [["run", "--level", "+1"], "--level: not a LevelNo"],
  [["run", "--level", "one"], "--level: not a LevelNo"],
  [["run", "--level", ""], "--level: not a LevelNo"],
  [["run", "--level", "1 "], "--level: not a LevelNo"],
  [["run", "--level", "١"], "--level: not a LevelNo"],
  [["run", "--level", "99999999999999999999999"], "--level: not a LevelNo"],
  [["run", "--level", "18446744073709551617"], "--level: not a LevelNo"],
  [["draw", "--at", "-1"], "--at: not a tick"],
  [["draw", "--at", "1.5"], "--at: not a tick"],
  [["draw", "--at", ""], "--at: not a tick"],
  [["draw", "--at", "1e3"], "--at: not a tick"],
  [["shot", "--out", ""], "--out: not a path"],
  [["run", "meadow"], 'unexpected argument "meadow"'],
  [["draw", "-"], 'unexpected argument "-"'],
  [["run", "--level", "7", "--fast"], "--level: not a LevelNo"],
  [["run", "--fast", "--level", "7"], "unknown flag --fast"],
])("refused: %j -> %s", (argv, message) => {
  expect(parse(argv)).toEqual(usage(message));
});

test("defaults", () => {
  expect(parse(["play"])).toEqual({ kind: "command", command: { name: "play", level: 1 } });
  expect(parse(["run"])).toEqual({ kind: "command", command: { name: "run", level: 1 } });
  expect(parse(["draw"])).toEqual({ kind: "command", command: { name: "draw", level: 1, at: null } });
  expect(parse(["shot"])).toEqual({ kind: "command", command: { name: "shot", level: 1, at: null, out: null } });
});

test("values: leading zeros, a tick of any length, a flag's value taken whatever it is", () => {
  expect(parse(["run", "--level", "0002"])).toEqual({ kind: "command", command: { name: "run", level: 2 } });
  expect(parse(["draw", "--at", "007", "--level", "2"])).toEqual({
    kind: "command",
    command: { name: "draw", level: 2, at: 7n },
  });
  expect(parse(["draw", "--at", "340282366920938463463374607431768211457"])).toEqual({
    kind: "command",
    command: { name: "draw", level: 1, at: 2n ** 128n + 1n },
  });
  expect(parse(["shot", "--out", "--level", "--at", "0"])).toEqual({
    kind: "command",
    command: { name: "shot", level: 1, at: 0n, out: "--level" },
  });
});
