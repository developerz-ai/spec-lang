// Runtime shim of the bun backend of spec-lang build: generators of base types. The same in every generated project; change the backend, not this file.

import fc from "fast-check";
import type { Claimed } from "../../src/runtime/values.ts";
import { setOf } from "../../src/runtime/values.ts";

/** Every property and law: this many runs from this seed, so each run of the suite is the same. */
export const RUNS = { seed: 20261124, numRuns: 200 } as const;

/** Events in the trace a law is checked on, and the most events run before it to reach its start world. */
export const TRACE_LENGTH = 40;
export const PREFIX_MAX = 20;

/** `Int`: mostly small, so numbers meet the ones a program holds; now and then past every machine word. */
export const intArb: fc.Arbitrary<bigint> = fc.oneof(
  { weight: 8, arbitrary: fc.bigInt(0n, 16n) },
  { weight: 1, arbitrary: fc.bigInt(-16n, -1n) },
  { weight: 1, arbitrary: fc.bigInt(-(2n ** 70n), 2n ** 70n) },
);

const WORD = [..."abcdefghijklmnopqrstuvwxyz0123456789-"];

/** `Text`: half plain words, so tight claims are met often; half anything, white space and astral characters included. */
export const textArb: fc.Arbitrary<string> = fc.oneof(
  fc.string({ unit: fc.constantFrom(...WORD), minLength: 1, maxLength: 8 }),
  fc.string({ unit: "grapheme", maxLength: 12 }),
  fc.constantFrom("", " ", "a b", "  padded  ", "　", "﻿", '"quoted" back\\slash', "x".repeat(200), "x".repeat(201)),
);

/** Values of a claimed type: values of its base that meet the claim. */
export function claimedArb<Base, T extends Base>(base: fc.Arbitrary<Base>, type: Claimed<Base, T>): fc.Arbitrary<T> {
  return base.map(type.parse).filter((value): value is T => value !== undefined);
}

export function setArb<T>(of: fc.Arbitrary<T>): fc.Arbitrary<readonly T[]> {
  return fc.array(of, { maxLength: 3 }).map(setOf);
}

export function optionArb<T>(of: fc.Arbitrary<T>): fc.Arbitrary<T | undefined> {
  return fc.option(of, { nil: undefined });
}
