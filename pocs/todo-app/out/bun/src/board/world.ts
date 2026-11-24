// Projected by hand from state tasks, state next_no (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { type Task, taskEquals } from "../model/task.ts";
import { FIRST_NO, type TaskNo } from "../model/task-no.ts";

/** Every `state` of todo.board. Immutable: a rule returns a new World. */
export interface World {
  readonly tasks: ReadonlyMap<TaskNo, Task>;
  readonly nextNo: TaskNo;
}

/** `tasks` starts empty, `next_no` at 1. */
export const EMPTY_WORLD: World = Object.freeze({ tasks: new Map<TaskNo, Task>(), nextNo: FIRST_NO });

export function withTask(w: World, t: Task): World {
  const tasks = new Map(w.tasks);
  tasks.set(t.no, t);
  return { tasks, nextNo: w.nextNo };
}

/** Structural `==` on the whole state. */
export function worldEquals(a: World, b: World): boolean {
  if (a.nextNo !== b.nextNo || a.tasks.size !== b.tasks.size) return false;
  for (const [no, t] of a.tasks) {
    const u = b.tasks.get(no);
    if (u === undefined || !taskEquals(t, u)) return false;
  }
  return true;
}
