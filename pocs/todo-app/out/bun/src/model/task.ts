// Projected by hand from Task (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Priority } from "./priority.ts";
import type { Status } from "./status.ts";
import type { TagSet } from "./tag.ts";
import type { TaskNo } from "./task-no.ts";
import type { Title } from "./title.ts";

export interface Task {
  readonly no: TaskNo;
  readonly title: Title;
  readonly status: Status;
  readonly priority: Priority;
  readonly tags: TagSet;
}

export interface TaskDefaults {
  readonly status?: Status;
  readonly priority?: Priority;
  readonly tags?: TagSet;
}

/** `Task(no, title: ..., status: Open, priority: Normal, tags: Set())` with the declared defaults. */
export function makeTask(no: TaskNo, title: Title, rest: TaskDefaults = {}): Task {
  return Object.freeze({
    no,
    title,
    status: rest.status ?? "Open",
    priority: rest.priority ?? "Normal",
    tags: rest.tags ?? Object.freeze([]),
  });
}

/** Structural `==`. */
export function taskEquals(a: Task, b: Task): boolean {
  return (
    a.no === b.no &&
    a.title === b.title &&
    a.status === b.status &&
    a.priority === b.priority &&
    a.tags.length === b.tags.length &&
    a.tags.every((t, i) => t === b.tags[i])
  );
}
