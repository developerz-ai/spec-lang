// Projected by hand from "Bytes written" and "Accepted on read" (docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

import { describe, expect, test } from "bun:test";
import fc from "fast-check";
import { EMPTY_WORLD, worldEquals } from "../../src/board/world.ts";
import { readCanonical } from "../../src/store/canonical-read.ts";
import { decodeWorld } from "../../src/store/decode.ts";
import { encodeWorld } from "../../src/store/encode.ts";
import { readJson } from "../../src/store/json-reader.ts";
import { apply, eventArb } from "../laws/commands.ts";
import { no, title } from "../support/values.ts";

/** The general path: strict JSON reader, then the schema. */
function general(text: string) {
  const j = readJson(text);
  return j.ok ? decodeWorld(j.value) : undefined;
}

describe("canonical fast path", () => {
  test("reads every store the writer can produce, equal to the general reader", () => {
    fc.assert(
      fc.property(fc.array(eventArb, { maxLength: 40 }), (events) => {
        const w = events.reduce((acc, e) => {
          const o = apply(acc, e);
          return o.ok ? o.world : acc;
        }, EMPTY_WORLD);
        const e = encodeWorld(w);
        if (!e.ok) throw new Error(e.error);
        const fast = readCanonical(e.text);
        const slow = general(e.text);
        expect(fast !== undefined && worldEquals(fast, w)).toBe(true);
        expect(slow?.ok === true && worldEquals(slow.world, w)).toBe(true);
      }),
      { seed: 20261124, numRuns: 300 },
    );
  });

  test.each([
    ["no newline", '{"version":1,"next_no":1,"tasks":[]}'],
    ["white space", '{"version":1,"next_no":1, "tasks":[]}\n'],
    [
      "tasks out of order",
      `{"version":1,"next_no":3,"tasks":[${'{"no":2,"title":"a","status":"Open","priority":"Normal","tags":[]}'},${'{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]}'}]}\n`,
    ],
    [
      "number twice",
      `{"version":1,"next_no":3,"tasks":[${'{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]}'},${'{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]}'}]}\n`,
    ],
    [
      "tags out of order",
      '{"version":1,"next_no":2,"tasks":[{"no":1,"title":"a","status":"Open","priority":"Normal","tags":["b","a"]}]}\n',
    ],
    [
      "tag starting with a dash",
      '{"version":1,"next_no":2,"tasks":[{"no":1,"title":"a","status":"Open","priority":"Normal","tags":["-x"]}]}\n',
    ],
    [
      "tag twice",
      '{"version":1,"next_no":2,"tasks":[{"no":1,"title":"a","status":"Open","priority":"Normal","tags":["a","a"]}]}\n',
    ],
    [
      "number not below next_no",
      '{"version":1,"next_no":1,"tasks":[{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]}]}\n',
    ],
    [
      "blank title",
      '{"version":1,"next_no":2,"tasks":[{"no":1,"title":" ","status":"Open","priority":"Normal","tags":[]}]}\n',
    ],
    [
      "a \\u escape",
      '{"version":1,"next_no":2,"tasks":[{"no":1,"title":"\\u0061","status":"Open","priority":"Normal","tags":[]}]}\n',
    ],
    ["next_no above 2^53", '{"version":1,"next_no":9007199254740993,"tasks":[]}\n'],
    ["next_no with a leading zero", '{"version":1,"next_no":01,"tasks":[]}\n'],
    [
      "a trailing comma",
      '{"version":1,"next_no":2,"tasks":[{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]},]}\n',
    ],
    ["trailing garbage", '{"version":1,"next_no":1,"tasks":[]}\nx'],
  ])("leaves %s to the general reader", (_, text) => {
    expect(readCanonical(text)).toBeUndefined();
  });

  test("an escaped quote and backslash in a title", () => {
    const w = readCanonical(
      '{"version":1,"next_no":2,"tasks":[{"no":1,"title":"a \\"b\\" \\\\ c","status":"Open","priority":"Normal","tags":[]}]}\n',
    );
    expect(w?.tasks.get(no(1))?.title).toBe(title('a "b" \\ c'));
  });
});
