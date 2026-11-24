// Projected by hand from rule done, rule reopen (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { TaskNo } from "../model/task-no.ts";
import { line } from "../render/line.ts";
import { commit, type Outcome, refuse } from "./outcome.ts";
import { type World, withTask } from "./world.ts";

/** `@guard(no in tasks else NoTask, tasks[no].status == Open else AlreadyDone)` */
export function done(w: World, no: TaskNo): Outcome {
  const t = w.tasks.get(no);
  if (t === undefined) return refuse("NoTask");
  if (t.status !== "Open") return refuse("AlreadyDone");
  const changed = Object.freeze({ ...t, status: "Done" as const });
  return commit(withTask(w, changed), [line(changed)], undefined);
}

/** `@guard(no in tasks else NoTask, tasks[no].status == Done else NotDone)` */
export function reopen(w: World, no: TaskNo): Outcome {
  const t = w.tasks.get(no);
  if (t === undefined) return refuse("NoTask");
  if (t.status !== "Done") return refuse("NotDone");
  const changed = Object.freeze({ ...t, status: "Open" as const });
  return commit(withTask(w, changed), [line(changed)], undefined);
}
