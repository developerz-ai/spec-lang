// Projected by hand from @store(file(".todo.json")): "Bytes written", "Accepted on read" (docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

import { describe, expect, test } from "bun:test";
import fc from "fast-check";
import { EMPTY_WORLD, type World, worldEquals } from "../../src/board/world.ts";
import type { Task } from "../../src/model/task.ts";
import type { TaskNo } from "../../src/model/task-no.ts";
import { encodeWorld } from "../../src/store/encode.ts";
import { parseStore } from "../../src/store/read-store.ts";
import { apply, eventArb } from "../laws/commands.ts";
import { no, tags, task } from "../support/values.ts";
import { threeOpenOneDone } from "../support/worlds.ts";

const utf8 = (s: string) => new TextEncoder().encode(s);

function worldOf(nextNo: number, ...list: Task[]): World {
  return { tasks: new Map<TaskNo, Task>(list.map((t) => [t.no, t])), nextNo: no(nextNo) };
}

function encoded(w: World): string {
  const e = encodeWorld(w);
  if (!e.ok) throw new Error(e.error);
  return e.text;
}

describe("canonical writer", () => {
  test("the contract's example, byte for byte", () => {
    const w = worldOf(
      3,
      task(1, "milk"),
      task(2, 'bread "fresh"', { status: "Done", priority: "High", tags: tags("shop", "home") }),
    );
    expect(encoded(w)).toBe(
      '{"version":1,"next_no":3,"tasks":[{"no":1,"title":"milk","status":"Open","priority":"Normal","tags":[]},{"no":2,"title":"bread \\"fresh\\"","status":"Done","priority":"High","tags":["home","shop"]}]}\n',
    );
  });

  test("the empty world", () => {
    expect(encoded(EMPTY_WORLD)).toBe('{"version":1,"next_no":1,"tasks":[]}\n');
  });

  test("escapes only quote and backslash; everything else is raw UTF-8", () => {
    const w = worldOf(2, task(1, 'a\u2027b\u0084c "q" \\ é 😀 ﻿'));
    expect(encoded(w)).toContain('"title":"a\u2027b\u0084c \\"q\\" \\\\ é 😀 ﻿"');
  });

  test("tasks by number, whatever order they were added in", () => {
    const w = worldOf(11, task(10, "b"), task(2, "a"));
    expect(encoded(w)).toContain('"tasks":[{"no":2,');
  });

  test("next_no 2^53 is the last that fits; past it the store is full", () => {
    expect(encodeWorld({ tasks: new Map(), nextNo: no(2n ** 53n) }).ok).toBe(true);
    expect(encodeWorld({ tasks: new Map(), nextNo: no(2n ** 53n + 1n) })).toEqual({ ok: false, error: "full" });
  });
});

describe("round trip", () => {
  test("three_open_one_done", () => {
    const w = threeOpenOneDone();
    const r = parseStore(utf8(encoded(w)));
    expect(r.ok && worldEquals(r.world, w)).toBe(true);
  });

  test("any world the rules reach reads back equal, and writes back the same bytes", () => {
    fc.assert(
      fc.property(fc.array(eventArb, { maxLength: 40 }), (events) => {
        const w = events.reduce((acc, e) => {
          const o = apply(acc, e);
          return o.ok ? o.world : acc;
        }, EMPTY_WORLD);
        const text = encoded(w);
        const r = parseStore(utf8(text));
        expect(r.ok && worldEquals(r.world, w)).toBe(true);
        expect(r.ok && encoded(r.world)).toBe(text);
      }),
      { seed: 20261124, numRuns: 200 },
    );
  });

  test("a readable store that is not canonical", () => {
    const r = parseStore(
      utf8(
        '{ "tasks" : [ {"tags":["b","a"],"priority":"High","status":"Done","title":"caf\\u00e9 \\ud83d\\ude00","no":2}, {"no":1,"title":"x","status":"Open","priority":"Low","tags":[]} ], "next_no" : 7, "version" : 1 }\n',
      ),
    );
    expect(r.ok && encoded(r.world)).toBe(
      '{"version":1,"next_no":7,"tasks":[{"no":1,"title":"x","status":"Open","priority":"Low","tags":[]},{"no":2,"title":"café 😀","status":"Done","priority":"High","tags":["a","b"]}]}\n',
    );
  });
});

describe("unreadable", () => {
  const T = (fields: string) => `{"version":1,"next_no":2,"tasks":[{${fields}}]}`;
  const ok = '"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]';
  test.each([
    ["blank", ""],
    ["empty object", "{}"],
    ["an array", "[1]"],
    ["version 2", '{"version":2,"next_no":1,"tasks":[]}'],
    ["version 1.0", '{"version":1.0,"next_no":1,"tasks":[]}'],
    ["next_no zero", '{"version":1,"next_no":0,"tasks":[]}'],
    ["next_no negative", '{"version":1,"next_no":-1,"tasks":[]}'],
    ["next_no 1.0", '{"version":1,"next_no":1.0,"tasks":[]}'],
    ["next_no 1e0", '{"version":1,"next_no":1e0,"tasks":[]}'],
    ["next_no a string", '{"version":1,"next_no":"1","tasks":[]}'],
    ["next_no above 2^53", '{"version":1,"next_no":9007199254740993,"tasks":[]}'],
    ["tasks an object", '{"version":1,"next_no":1,"tasks":{}}'],
    ["tasks holds a number", '{"version":1,"next_no":1,"tasks":[1]}'],
    ["extra top-level key", '{"version":1,"next_no":1,"tasks":[],"x":0}'],
    ["missing key", '{"version":1,"next_no":1}'],
    ["duplicate key", '{"version":1,"next_no":1,"next_no":1,"tasks":[]}'],
    ["duplicate key in a task", T(`"no":1,${ok}`)],
    [
      "number not below next_no",
      '{"version":1,"next_no":1,"tasks":[{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]}]}',
    ],
    ["number zero", T('"no":0,"title":"a","status":"Open","priority":"Normal","tags":[]')],
    ["number twice", `{"version":1,"next_no":2,"tasks":[{${ok}},{${ok}}]}`],
    ["blank title", T('"no":1,"title":" ","status":"Open","priority":"Normal","tags":[]')],
    ["title with a tab", T('"no":1,"title":"a\\tb","status":"Open","priority":"Normal","tags":[]')],
    ["title a lone surrogate", T('"no":1,"title":"\\ud800","status":"Open","priority":"Normal","tags":[]')],
    ["title a number", T('"no":1,"title":1,"status":"Open","priority":"Normal","tags":[]')],
    ["status lower case", T('"no":1,"title":"a","status":"open","priority":"Normal","tags":[]')],
    ["priority unknown", T('"no":1,"title":"a","status":"Open","priority":"Urgent","tags":[]')],
    ["bad tag", T('"no":1,"title":"a","status":"Open","priority":"Normal","tags":["Home"]')],
    ["tag starting with a dash", T('"no":1,"title":"a","status":"Open","priority":"Normal","tags":["-x"]')],
    ["title with a line separator", T('"no":1,"title":"a\u2028b","status":"Open","priority":"Normal","tags":[]')],
    ["title with an escaped next line", T('"no":1,"title":"a\\u0085b","status":"Open","priority":"Normal","tags":[]')],
    ["tag twice", T('"no":1,"title":"a","status":"Open","priority":"Normal","tags":["a","a"]')],
    ["tag twice, not adjacent", T('"no":1,"title":"a","status":"Open","priority":"Normal","tags":["b","a","b"]')],
    ["tags not an array", T('"no":1,"title":"a","status":"Open","priority":"Normal","tags":"a"')],
    ["extra task key", T(`${ok},"due":null`)],
    ["missing task key", T('"no":1,"title":"a","status":"Open","priority":"Normal"')],
    ["trailing garbage", '{"version":1,"next_no":1,"tasks":[]} x'],
    ["not JSON", "todo"],
  ])("%s", (_, text) => {
    expect(parseStore(utf8(text)).ok).toBe(false);
  });

  test("invalid UTF-8", () => {
    const bytes = new Uint8Array([
      ...utf8('{"version":1,"next_no":2,"tasks":[{"no":1,"title":"'),
      0xc3,
      0x28,
      ...utf8('","status":"Open","priority":"Normal","tags":[]}]}'),
    ]);
    expect(parseStore(bytes).ok).toBe(false);
  });

  test("a byte-order mark", () => {
    expect(parseStore(utf8('﻿{"version":1,"next_no":1,"tasks":[]}')).ok).toBe(false);
  });

  test("next_no exactly 2^53 is readable", () => {
    expect(parseStore(utf8('{"version":1,"next_no":9007199254740992,"tasks":[]}')).ok).toBe(true);
  });
});
