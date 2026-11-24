// Projected by hand from the trace events board.add .. board.stats (src/todo/laws.slg, src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

import fc from "fast-check";
import { add } from "../../src/board/add.ts";
import { clear } from "../../src/board/clear.ts";
import { done, reopen } from "../../src/board/done.ts";
import { list } from "../../src/board/list.ts";
import type { Outcome } from "../../src/board/outcome.ts";
import { remove } from "../../src/board/remove.ts";
import { rename } from "../../src/board/rename.ts";
import { stats } from "../../src/board/stats.ts";
import type { World } from "../../src/board/world.ts";
import type { Priority } from "../../src/model/priority.ts";
import type { Tag, TagSet } from "../../src/model/tag.ts";
import type { TaskNo } from "../../src/model/task-no.ts";
import type { Title } from "../../src/model/title.ts";
import { no, tag, tags, title } from "../support/values.ts";

/** One event of a trace: a rule and its parameters. */
export type Event =
  | { readonly rule: "add"; readonly title: Title; readonly priority: Priority; readonly tags: TagSet }
  | { readonly rule: "list"; readonly all: boolean; readonly tag: Tag | undefined }
  | { readonly rule: "done" | "reopen" | "remove"; readonly no: TaskNo }
  | { readonly rule: "rename"; readonly no: TaskNo; readonly title: Title }
  | { readonly rule: "clear" | "stats" };

const titleArb = fc.constantFrom("milk", "  café ☕ ", "﻿", "a\u2027b", "x".repeat(200)).map(title);
const tagArb = fc.constantFrom("home", "work", "a-1").map(tag);
/** Small numbers, so most events name a task that exists; now and then a huge one that cannot. */
const noArb = fc
  .oneof({ weight: 9, arbitrary: fc.bigInt(1n, 16n) }, { weight: 1, arbitrary: fc.bigInt(1n, 2n ** 70n) })
  .map(no);

export const eventArb: fc.Arbitrary<Event> = fc.oneof(
  {
    weight: 4,
    arbitrary: fc.record({
      rule: fc.constant("add" as const),
      title: titleArb,
      priority: fc.constantFrom<Priority>("Low", "Normal", "High"),
      tags: fc.uniqueArray(tagArb, { maxLength: 3 }).map((xs) => tags(...xs)),
    }),
  },
  {
    weight: 1,
    arbitrary: fc.record({
      rule: fc.constant("list" as const),
      all: fc.boolean(),
      tag: fc.option(tagArb, { nil: undefined }),
    }),
  },
  {
    weight: 2,
    arbitrary: fc.record({ rule: fc.constantFrom("done" as const, "reopen" as const, "remove" as const), no: noArb }),
  },
  { weight: 1, arbitrary: fc.record({ rule: fc.constant("rename" as const), no: noArb, title: titleArb }) },
  { weight: 1, arbitrary: fc.record({ rule: fc.constantFrom("clear" as const, "stats" as const) }) },
);

/** 40 random commands, as `docs/contract.md` section 4 asks. */
export const traceArb = fc.array(eventArb, { minLength: 40, maxLength: 40 });

export function apply(w: World, e: Event): Outcome<unknown> {
  switch (e.rule) {
    case "add":
      return add(w, e.title, e.priority, e.tags);
    case "list":
      return list(w, e.all, e.tag);
    case "done":
      return done(w, e.no);
    case "reopen":
      return reopen(w, e.no);
    case "rename":
      return rename(w, e.no, e.title);
    case "remove":
      return remove(w, e.no);
    case "clear":
      return clear(w);
    case "stats":
      return stats(w);
  }
}
