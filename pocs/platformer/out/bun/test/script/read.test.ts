// Projected by hand from the input script derived from Pad, the parameter of step (docs/game-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import { pad } from "../../src/model/pad.ts";
import { type Read, readScript } from "../../src/script/read.ts";

const read = (text: string) => readScript(new TextEncoder().encode(text));
const bad = (message: string): Read => ({ ok: false, message });

test("comments, blank lines, tabs, leading zeros, a last line without a newline", () => {
  expect(read("# wait, then walk\n\n  002\t-   # two ticks of nothing\n0003 R")).toEqual({
    ok: true,
    script: [
      { ticks: 2, pad: pad() },
      { ticks: 3, pad: pad({ right: true }) },
    ],
  });
  expect(read("")).toEqual({ ok: true, script: [] });
});

test("every key, in any order", () => {
  expect(read("1 JBFRL\n100000 FL\n")).toEqual({
    ok: true,
    script: [
      { ticks: 1, pad: { left: true, right: true, forward: true, back: true, jump: true } },
      { ticks: 100000, pad: pad({ left: true, forward: true }) },
    ],
  });
});

test.each([
  ["5\n", "input line 1: expected <ticks> <keys>"],
  ["5 R J\n", "input line 1: expected <ticks> <keys>"],
  ["0 R\n", "input line 1: ticks must be 1 to 100000"],
  ["000 R\n", "input line 1: ticks must be 1 to 100000"],
  ["100001 R\n", "input line 1: ticks must be 1 to 100000"],
  ["99999999999999999999999999 R\n", "input line 1: ticks must be 1 to 100000"],
  ["-3 R\n", "input line 1: ticks must be 1 to 100000"],
  ["R 3\n", "input line 1: ticks must be 1 to 100000"],
  ["0 x\n", "input line 1: ticks must be 1 to 100000"],
  ["1 r\n", 'input line 1: unknown key "r"'],
  ["1 R-\n", 'input line 1: unknown key "-"'],
  ["1 --\n", 'input line 1: unknown key "-"'],
  ["1 RRZ\n", 'input line 1: key "R" twice'],
  ["1 RZR\n", 'input line 1: unknown key "Z"'],
  ["1 RJR\n", 'input line 1: key "R" twice'],
  ["1 R\r\n", 'input line 1: unknown key "\r"'],
  ["# header\n\n3 R\n2 X\n0 R\n", 'input line 4: unknown key "X"'],
  ["300 R\n1 Q\n", 'input line 2: unknown key "Q"'],
  ["1 R\n1 ÿ\n", 'input line 2: unknown key "ÿ"'],
  ["1 😀\n", 'input line 1: unknown key "😀"'],
])("refused: %j", (text, message) => {
  expect(read(text)).toEqual(bad(message));
});

test("invalid UTF-8 is an unknown key at its line", () => {
  expect(readScript(Uint8Array.of(0x31, 0x20, 0x52, 0x0a, 0x31, 0x20, 0xff, 0x0a))).toEqual(
    bad('input line 2: unknown key "�"'),
  );
});

test("at most a million ticks in all, and a bad line is reported first", () => {
  const million = "100000 R\n".repeat(10);
  expect(read(million).ok).toBe(true);
  expect(read(`${million}1 R\n`)).toEqual(bad("input: more than 1000000 ticks"));
  expect(read(`${million}1 R\n1 Q\n`)).toEqual(bad('input line 12: unknown key "Q"'));
});
