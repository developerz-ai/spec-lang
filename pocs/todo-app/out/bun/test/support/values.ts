// Projected by hand from literals of Title, Tag, TaskNo, Task in tests (src/todo/model.slg, src/todo/tests/scenarios.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { asTag, type Tag, type TagSet, tagSet } from "../../src/model/tag.ts";
import { makeTask, type Task, type TaskDefaults } from "../../src/model/task.ts";
import { asTaskNo, type TaskNo } from "../../src/model/task-no.ts";
import { asTitle, type Title } from "../../src/model/title.ts";

/** A literal in a test is checked against its type's claim, as the build would. */
function claim<T>(value: T | undefined, what: string): T {
  if (value === undefined) throw new Error(`literal is not a ${what}`);
  return value;
}

export const title = (s: string): Title => claim(asTitle(s), "Title");
export const tag = (s: string): Tag => claim(asTag(s), "Tag");
export const no = (n: number | bigint): TaskNo => claim(asTaskNo(BigInt(n)), "TaskNo");
export const tags = (...xs: string[]): TagSet => tagSet(xs.map(tag));

/** `Task(n, title: "...", ...)` */
export function task(n: number, t: string, rest: TaskDefaults = {}): Task {
  return makeTask(no(n), title(t), rest);
}
