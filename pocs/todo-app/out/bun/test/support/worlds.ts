// Projected by hand from test three_open_one_done (src/todo/tests/scenarios.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { World } from "../../src/board/world.ts";
import type { Task } from "../../src/model/task.ts";
import type { TaskNo } from "../../src/model/task-no.ts";
import { no, tags, task } from "./values.ts";

/** The `@given` of three_open_one_done. */
export function threeOpenOneDone(): World {
  const list: Task[] = [
    task(1, "low one", { priority: "Low" }),
    task(2, "plain"),
    task(3, "urgent", { priority: "High", tags: tags("work") }),
    task(4, "finished", { status: "Done", tags: tags("work", "home") }),
  ];
  return { tasks: new Map<TaskNo, Task>(list.map((t) => [t.no, t])), nextNo: no(5) };
}
