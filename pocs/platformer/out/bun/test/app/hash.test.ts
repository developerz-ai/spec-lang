// Projected by hand from the pixel hash of shot: FNV-1a 64 (docs/game-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import fc from "fast-check";
import { fnv1a64 } from "../../src/app/hash.ts";

/** The definition, on BigInt. */
function reference(bytes: Uint8Array): string {
  let h = 0xcbf29ce484222325n;
  for (const b of bytes) h = ((h ^ BigInt(b)) * 0x100000001b3n) & 0xffffffffffffffffn;
  return h.toString(16).padStart(16, "0");
}

test("published vectors", () => {
  expect(fnv1a64(new Uint8Array())).toBe("cbf29ce484222325");
  expect(fnv1a64(new TextEncoder().encode("a"))).toBe("af63dc4c8601ec8c");
  expect(fnv1a64(new TextEncoder().encode("foobar"))).toBe("85944171f73967e8");
});

test("the two-halves arithmetic equals the BigInt definition", () => {
  fc.assert(
    fc.property(fc.uint8Array({ maxLength: 4096 }), (bytes) => {
      expect(fnv1a64(bytes)).toBe(reference(bytes));
    }),
    { seed: 20261124, numRuns: 300 },
  );
  expect(fnv1a64(new Uint8Array(320 * 180 * 4).fill(255))).toBe(reference(new Uint8Array(320 * 180 * 4).fill(255)));
});
