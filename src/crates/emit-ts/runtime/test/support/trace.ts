import type { RuleOutcome } from "../../src/runtime/outcome.ts";

/** One element of `t.steps`. */
export interface Step<W, E> {
  readonly event: E;
  readonly before: W;
  readonly after: W;
  readonly reply: RuleOutcome<W, string, unknown>;
}

/** A trace: the states it passed through, the first included, and the steps between them. */
export interface Trace<W, E> {
  readonly states: readonly W[];
  readonly steps: readonly Step<W, E>[];
}

type Apply<W, E> = (world: W, event: E) => RuleOutcome<W, string, unknown>;

/** The world after `events`; a refused event leaves it as it was. Every such world is one the rules can reach. */
export function reach<W, E>(start: W, events: readonly E[], apply: Apply<W, E>): W {
  let world = start;
  for (const event of events) {
    const reply = apply(world, event);
    if (reply.ok) world = reply.world;
  }
  return world;
}

export function runTrace<W, E>(start: W, events: readonly E[], apply: Apply<W, E>): Trace<W, E> {
  const states = [start];
  const steps: Step<W, E>[] = [];
  let world = start;
  for (const event of events) {
    const reply = apply(world, event);
    const after = reply.ok ? reply.world : world;
    steps.push({ event, before: world, after, reply });
    states.push(after);
    world = after;
  }
  return { states, steps };
}
