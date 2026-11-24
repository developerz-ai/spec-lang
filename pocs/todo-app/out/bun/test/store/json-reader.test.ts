// Projected by hand from "Accepted on read" (docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

import { describe, expect, test } from "bun:test";
import { readJson } from "../../src/store/json-reader.ts";

describe("strict JSON reader", () => {
  test("keeps integers exact past 2^53", () => {
    const r = readJson("[9007199254740993, 123456789012345678901234567890]");
    expect(r.ok && r.value.kind === "array" ? r.value.items : []).toEqual([
      { kind: "number", integer: 9007199254740993n },
      { kind: "number", integer: 123456789012345678901234567890n },
    ]);
  });

  test("a fraction or an exponent is a number but not an integer", () => {
    for (const t of ["1.0", "1e0", "1E+2", "-0.5"]) {
      const r = readJson(t);
      expect(r.ok && r.value.kind === "number" && r.value.integer === undefined).toBe(true);
    }
  });

  test("joins surrogate pairs and reads every escape", () => {
    const r = readJson('"caf\\u00e9 \\ud83d\\ude00 \\"\\\\\\/\\b\\f\\n\\r\\t"');
    expect(r.ok && r.value.kind === "string" ? r.value.value : null).toBe('café 😀 "\\/\b\f\n\r\t');
  });

  test.each([
    ["", "empty"],
    [" ", "blank"],
    ['{"a":1,"a":1}', "duplicate key"],
    ['{"a":{"b":1,"b":2}}', "duplicate key, nested"],
    ['"\\ud800"', "lone high surrogate"],
    ['"\\udc00"', "lone low surrogate"],
    ['"\\ud800\\u0041"', "high surrogate then not a low one"],
    ["01", "leading zero"],
    ["+1", "plus sign"],
    ["1.", "bare point"],
    [".5", "no integer part"],
    ["NaN", "not JSON"],
    ["[1,]", "trailing comma"],
    ['{"a":1,}', "trailing comma in object"],
    ['"a\tb"', "raw control character"],
    ['"\\x"', "unknown escape"],
    ['"\\u12"', "short \\u escape"],
    ["{} x", "trailing garbage"],
    ["﻿{}", "byte-order mark"],
    [" {}", "non-JSON white space"],
    ["[".repeat(100) + "]".repeat(100), "nested too deep"],
    ["tru", "cut literal"],
    ['"abc', "unterminated string"],
  ])("refuses %j (%s)", (text) => {
    expect(readJson(text).ok).toBe(false);
  });
});
