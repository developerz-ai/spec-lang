// Projected by hand from every law in todo.laws (src/todo/laws.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Outcome } from "../../src/board/outcome.ts";
import { type World, worldEquals } from "../../src/board/world.ts";
import { taskEquals } from "../../src/model/task.ts";
import { asTaskNo, type TaskNo } from "../../src/model/task-no.ts";
import type { Event } from "./commands.ts";

/** One element of `t.steps`. */
export interface Step {
  readonly event: Event;
  readonly before: World;
  readonly after: World;
  readonly reply: Outcome<unknown>;
}

const CHANGES: ReadonlySet<Event["rule"]> = new Set(["add", "done", "reopen", "rename", "remove", "clear"]);
const NAMES_A_TASK: ReadonlySet<Event["rule"]> = new Set(["done", "reopen", "rename", "remove"]);
const READS: ReadonlySet<Event["rule"]> = new Set(["list", "stats"]);

/** `t.states.all(w => w.tasks.keys.all(it < w.next_no))` */
export function numbersBelowNext(w: World): boolean {
  return [...w.tasks.keys()].every((k) => k < w.nextNo);
}

/** `t.steps.all(it.after.next_no >= it.before.next_no)` over Change */
export function nextOnlyGrows(s: Step): boolean {
  return !CHANGES.has(s.event.rule) || s.after.nextNo >= s.before.nextNo;
}

/** Every k the step could disturb: each key before or after, and the numbers around them. */
function keysToCheck(s: Step): TaskNo[] {
  const ks = new Set<bigint>([...s.before.tasks.keys(), ...s.after.tasks.keys(), s.before.nextNo, s.after.nextNo, 1n]);
  return [...ks].flatMap((k) => [asTaskNo(k), asTaskNo(k + 1n)]).filter((k): k is TaskNo => k !== undefined);
}

/** `s.event.no != k implies s.after.tasks.get(k) == s.before.tasks.get(k)`, for every k */
export function othersUntouched(s: Step): boolean {
  const e = s.event;
  if (!("no" in e) || !NAMES_A_TASK.has(e.rule)) return true;
  return keysToCheck(s).every((k) => {
    if (k === e.no) return true;
    const a = s.after.tasks.get(k);
    const b = s.before.tasks.get(k);
    return a === undefined || b === undefined ? a === b : taskEquals(a, b);
  });
}

/** After clear every task is open, and exactly the open ones stay. */
export function clearKeepsEveryOpenTask(s: Step): boolean {
  if (s.event.rule !== "clear") return true;
  const after = [...s.after.tasks.values()];
  const openBefore = [...s.before.tasks.values()].filter((t) => t.status === "Open");
  return after.every((t) => t.status === "Open") && s.after.tasks.size === openBefore.length;
}

/** `t.steps.all(it.after == it.before)` over list, stats */
export function readsChangeNothing(s: Step): boolean {
  return !READS.has(s.event.rule) || worldEquals(s.after, s.before);
}

/** `s.reply.is(fail(_)) implies s.after == s.before` over Change */
export function refusalChangesNothing(s: Step): boolean {
  return !CHANGES.has(s.event.rule) || s.reply.ok || worldEquals(s.after, s.before);
}
