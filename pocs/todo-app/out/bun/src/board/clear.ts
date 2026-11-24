// Projected by hand from rule clear (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { commit, type Outcome } from "./outcome.ts";
import type { World } from "./world.ts";

/** Finished work leaves the list in one step. Replies with how many tasks went. */
export function clear(w: World): Outcome<number> {
  const tasks = new Map(w.tasks);
  let gone = 0;
  for (const t of w.tasks.values()) {
    if (t.status === "Done") {
      tasks.delete(t.no);
      gone++;
    }
  }
  return commit({ tasks, nextNo: w.nextNo }, [`cleared ${gone}`], gone);
}
