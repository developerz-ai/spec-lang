// Projected by hand from rule add (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Priority } from "../model/priority.ts";
import type { TagSet } from "../model/tag.ts";
import { makeTask } from "../model/task.ts";
import { nextNo, type TaskNo } from "../model/task-no.ts";
import type { Title } from "../model/title.ts";
import { commit, type Outcome } from "./outcome.ts";
import { type World, withTask } from "./world.ts";

/** A number is never reused: `next_no` only grows. */
export function add(w: World, title: Title, priority: Priority = "Normal", tags: TagSet = []): Outcome<TaskNo> {
  const no = w.nextNo;
  const added = withTask(w, makeTask(no, title, { priority, tags }));
  return commit({ tasks: added.tasks, nextNo: nextNo(no) }, [`added ${no}`], no);
}
