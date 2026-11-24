// Projected by hand from app SkyhopBun: help, usage errors, run and draw, end to end in process (src/apps/bun.slg, docs/game-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { execute, type Io } from "../../src/app/execute.ts";
import { HELP } from "../../src/cli/help.ts";

const inputs = resolve(import.meta.dir, "../../../../parity/inputs");

/** Standard input given as text; a command that must not read it fails the test if it does. */
function io(stdin: string | null): Io {
  return {
    env: {},
    stdin: () => {
      if (stdin === null) throw new Error("standard input was read");
      return new TextEncoder().encode(stdin);
    },
    write: () => {
      throw new Error("only play writes as it goes");
    },
  };
}

test("help: the contract's bytes, to standard error without arguments and to standard output when asked", () => {
  expect(HELP.split("\n").length).toBe(11);
  expect(execute([], io(null))).toEqual({ stdout: "", stderr: HELP, code: 2 });
  expect(execute(["run", "--level", "9", "-h"], io(null))).toEqual({ stdout: HELP, stderr: "", code: 0 });
});

test("a usage error is one line, exit 2, before any input is read", () => {
  expect(execute(["run", "--level", "7", "--fast"], io(null))).toEqual({
    stdout: "",
    stderr: "error: --level: not a LevelNo\n",
    code: 2,
  });
});

test("a bad script is refused whole: nothing on standard output", () => {
  const refused = { stdout: "", stderr: 'error: input line 2: unknown key "Q"\n', code: 2 };
  expect(execute(["run"], io("300 R\n1 Q\n"))).toEqual(refused);
  expect(execute(["draw"], io("300 R\n1 Q\n"))).toEqual(refused);
  // shot refuses it before any window opens: the environment here names no library and no display.
  expect(execute(["shot"], io("300 R\n1 Q\n"))).toEqual(refused);
});

test("run: an empty script still starts the level", () => {
  expect(execute(["run"], io(""))).toEqual({
    stdout: "0 level meadow\nend 0 playing gems 0/3 lives 3 at 0,0,0\n",
    stderr: "",
    code: 0,
  });
});

test("run: the meadow, won with every gem; ticks after the flag are not played", () => {
  const script = readFileSync(resolve(inputs, "meadow-win.txt"), "utf8");
  const won = execute(["run", "--level", "001"], io(script));
  expect(won.stdout.split("\n").slice(-4)).toEqual([
    "284 gem 3",
    "316 won",
    "end 316 won gems 3/3 lives 3 at 4992,0,0",
    "",
  ]);
  expect(execute(["run"], io(`${script}100000 L\n`))).toEqual(won);
});

test("run: the ridge", () => {
  const out = execute(["run", "--level", "2"], io(readFileSync(resolve(inputs, "ridge-win.txt"), "utf8"))).stdout;
  expect(out.startsWith("0 level ridge\n")).toBe(true);
  expect(
    out.endsWith("549 won\nend 549 won gems 3/3 lives 3 at 6088,320,0\n") ||
      out.includes("\n549 won\nend 549 won gems 3/3 lives 3 "),
  ).toBe(true);
});

test("run: a million ticks of falling ends lost, after three falls", () => {
  const out = execute(["run"], io("100000 R\n".repeat(10))).stdout.split("\n");
  expect(out.filter((l) => l.endsWith(" fall")).length).toBe(3);
  expect(out.at(-2)).toMatch(/^end \d+ lost gems 0\/3 lives 0 at 0,0,0$/);
});

test("draw: tick 0, and --at stops the script early or not at all", () => {
  const frame0 = execute(["draw", "--at", "0"], io("50 R\n")).stdout;
  expect(frame0).toBe(execute(["draw"], io("")).stdout);
  expect(frame0.split("\n").slice(0, 3)).toEqual(["sky 135,206,235", "camera 0,704,2048 0,192,0", "time 0"]);
  expect(
    frame0.endsWith("mesh pip 0,0,0 0\nlabel 16 12 20 255,255,255 gems 0/3\nlabel 16 38 20 255,255,255 lives 3\n"),
  ).toBe(true);

  const at5 = execute(["draw", "--at", "5"], io("50 R\n")).stdout;
  expect(at5).toContain("\ntime 5\n");
  const whole = execute(["draw"], io("50 R\n")).stdout;
  expect(whole).toContain("\ntime 50\n");
  expect(execute(["draw", "--at", "99999999999999999999999999999999999999"], io("50 R\n")).stdout).toBe(whole);
});

test("draw: the banner after the flag", () => {
  const script = readFileSync(resolve(inputs, "meadow-win.txt"), "utf8");
  expect(execute(["draw"], io(script)).stdout.endsWith("label 380 250 40 255,255,255 level clear\n")).toBe(true);
});
