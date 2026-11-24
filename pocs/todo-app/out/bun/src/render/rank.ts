// Projected by hand from urgency, rank (src/todo/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Priority } from "../model/priority.ts";
import type { Task } from "../model/task.ts";

const URGENCY: Readonly<Record<Priority, number>> = Object.freeze({ High: 0, Normal: 1, Low: 2 });

export function urgency(p: Priority): number {
  return URGENCY[p];
}

/** `rank(t) = (urgency(t.priority), t.no)`, compared as a tuple: most urgent first, then oldest first. */
export function compareRank(a: Task, b: Task): number {
  const d = urgency(a.priority) - urgency(b.priority);
  if (d !== 0) return d;
  return a.no < b.no ? -1 : a.no > b.no ? 1 : 0;
}
