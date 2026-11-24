// Projected by hand from every law in todo.laws as seeded property tests (src/todo/laws.slg, docs/contract.md section 4). Stand-in for spec-lang build output; change the spec, not this file.

import { describe, expect, test } from "bun:test";
import fc from "fast-check";
import { EMPTY_WORLD, type World, worldEquals } from "../../src/board/world.ts";
import { apply, type Event, traceArb } from "./commands.ts";
import {
  clearKeepsEveryOpenTask,
  nextOnlyGrows,
  numbersBelowNext,
  othersUntouched,
  readsChangeNothing,
  refusalChangesNothing,
  type Step,
} from "./laws.ts";

/** 200 traces of 40 events each; the seed makes every run the same. */
const RUNS = { seed: 20261124, numRuns: 200 } as const;

/** A copy that shares nothing mutable with `w`, to see that a rule never edits the world it was given. */
function snapshot(w: World): World {
  return { tasks: new Map(w.tasks), nextNo: w.nextNo };
}

/** Runs the trace and calls `check` after every step with the state and the step. */
function everyStep(events: readonly Event[], check: (state: World, step: Step) => void): void {
  let world = EMPTY_WORLD;
  for (const event of events) {
    const kept = snapshot(world);
    const reply = apply(world, event);
    expect(worldEquals(world, kept)).toBe(true);
    const after = reply.ok ? reply.world : world;
    check(after, { event, before: world, after, reply });
    world = after;
  }
}

const LAWS: ReadonlyArray<readonly [string, (state: World, step: Step) => boolean]> = [
  ["numbers_below_next", (state) => numbersBelowNext(state)],
  ["next_only_grows", (_, step) => nextOnlyGrows(step)],
  ["others_untouched", (_, step) => othersUntouched(step)],
  ["clear_keeps_every_open_task", (_, step) => clearKeepsEveryOpenTask(step)],
  ["reads_change_nothing", (_, step) => readsChangeNothing(step)],
  ["refusal_changes_nothing", (_, step) => refusalChangesNothing(step)],
];

describe("todo.laws", () => {
  for (const [name, law] of LAWS) {
    test(name, () => {
      let steps = 0;
      fc.assert(
        fc.property(traceArb, (events) =>
          everyStep(events, (state, step) => {
            steps++;
            expect(law(state, step)).toBe(true);
          }),
        ),
        RUNS,
      );
      expect(steps).toBe(200 * 40);
    });
  }

  test("the traces reach every rule, every refusal and a clear that removes work", () => {
    const seen = new Set<string>();
    fc.assert(
      fc.property(traceArb, (events) =>
        everyStep(events, (_, step) => {
          seen.add(step.reply.ok ? step.event.rule : step.reply.refusal);
          if (step.event.rule === "clear" && step.reply.ok && step.reply.reply !== 0) seen.add("clear>0");
        }),
      ),
      RUNS,
    );
    const want = ["add", "list", "done", "reopen", "rename", "remove", "clear", "stats", "NoTask", "AlreadyDone"];
    expect([...seen].sort()).toEqual([...want, "NotDone", "clear>0"].sort());
  });
});
