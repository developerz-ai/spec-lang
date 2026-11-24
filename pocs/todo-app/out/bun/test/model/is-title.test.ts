// Projected by hand from @cases on is_title (src/todo/model.slg), with the Text rules of docs/contract.md section 1. Stand-in for spec-lang build output; change the spec, not this file.

import { describe, expect, test } from "bun:test";
import fc from "fast-check";
import { bytes, isText, size, trim } from "../../src/model/text.ts";
import { isTitle } from "../../src/model/title.ts";

describe("is_title @cases", () => {
  test.each([
    ["milk", true],
    ["", false],
    ["   ", false],
    ["a\tb", false],
    ["　", false],
    ["a\u2028b", false],
    ["a\u0085b", false],
  ] as const)("%j -> %p", (t, want) => {
    expect(isTitle(t)).toBe(want);
  });
});

describe("is_title, contract section 1", () => {
  test.each([
    ["\u0085", false],
    ["   ", false],
    ["a\nb", false],
    ["a\u007fb", false],
    ["﻿", true],
    ["​", true],
    ["᠎", true],
    ["a b\u0085c", false],
    ["a\u2029b", false],
    ["a\u2027b\u0084c", true],
    ["é".repeat(200), true],
    ["é".repeat(201), false],
    ["\u{1f600}".repeat(200), true],
    ["\u{1f600}".repeat(201), false],
    ["x".repeat(200), true],
    ["x".repeat(201), false],
    ["\ud800", false],
  ] as const)("%j -> %p", (t, want) => {
    expect(isTitle(t)).toBe(want);
  });
});

/** The claim exactly as written, without the ASCII fast path. */
function literal(t: string): boolean {
  return (
    isText(t) &&
    size(trim(t)) >= 1 &&
    size(t) <= 200 &&
    bytes(t).every((b) => b >= 32 && b !== 127) &&
    !t.includes("\u0085") &&
    !t.includes("\u2028") &&
    !t.includes("\u2029")
  );
}

test("the ASCII fast path decides exactly as the claim", () => {
  const unit = fc.oneof(
    fc.constantFrom(" ", "a", "\t", "\u007f", "\u0085", "\u2028", "\u2029", "　", "﻿", "\u{1f600}", "\ud800"),
    fc.string({ unit: "grapheme-ascii", maxLength: 3 }),
  );
  fc.assert(
    fc.property(fc.array(unit, { maxLength: 260 }), (parts) => {
      const t = parts.join("");
      expect(isTitle(t)).toBe(literal(t));
    }),
    { seed: 20261124, numRuns: 2000 },
  );
});
