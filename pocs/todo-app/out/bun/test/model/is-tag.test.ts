// Projected by hand from @cases on is_tag (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { describe, expect, test } from "bun:test";
import fc from "fast-check";
import { isTag } from "../../src/model/tag.ts";
import { bytes, size } from "../../src/model/text.ts";

describe("is_tag @cases", () => {
  test.each([
    ["home", true],
    ["deep-work", true],
    ["Home", false],
    ["", false],
    ["a b", false],
    ["-x", false],
    ["--all", false],
  ] as const)("%j -> %p", (t, want) => {
    expect(isTag(t)).toBe(want);
  });
});

test("is_tag: 32 characters is the most", () => {
  expect(isTag("a".repeat(32))).toBe(true);
  expect(isTag("a".repeat(33))).toBe(false);
});

/** The claim exactly as written. */
function literal(t: string): boolean {
  const n = size(t);
  return (
    1 <= n &&
    n <= 32 &&
    bytes(t).every((b) => (97 <= b && b <= 122) || (48 <= b && b <= 57) || b === 45) &&
    !t.startsWith("-")
  );
}

test("the regex decides exactly as the claim", () => {
  fc.assert(
    fc.property(fc.string({ unit: "binary", maxLength: 40 }), (t) => {
      expect(isTag(t)).toBe(literal(t));
    }),
    { seed: 20261124, numRuns: 2000 },
  );
});
