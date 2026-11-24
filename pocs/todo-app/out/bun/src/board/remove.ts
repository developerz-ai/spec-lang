// Projected by hand from rule remove (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { TaskNo } from "../model/task-no.ts";
import { commit, type Outcome, refuse } from "./outcome.ts";
import type { World } from "./world.ts";

/** `@guard(no in tasks else NoTask)`. `next_no` stays, so the number is never reused. */
export function remove(w: World, no: TaskNo): Outcome {
  if (!w.tasks.has(no)) return refuse("NoTask");
  const tasks = new Map(w.tasks);
  tasks.delete(no);
  return commit({ tasks, nextNo: w.nextNo }, [`removed ${no}`], undefined);
}
