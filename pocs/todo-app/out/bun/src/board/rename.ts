// Projected by hand from rule rename (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { TaskNo } from "../model/task-no.ts";
import type { Title } from "../model/title.ts";
import { line } from "../render/line.ts";
import { commit, type Outcome, refuse } from "./outcome.ts";
import { type World, withTask } from "./world.ts";

/** `@guard(no in tasks else NoTask)` */
export function rename(w: World, no: TaskNo, title: Title): Outcome {
  const t = w.tasks.get(no);
  if (t === undefined) return refuse("NoTask");
  const changed = Object.freeze({ ...t, title });
  return commit(withTask(w, changed), [line(changed)], undefined);
}
