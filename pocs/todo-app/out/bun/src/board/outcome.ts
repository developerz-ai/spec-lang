// Projected by hand from fail NoTask, fail AlreadyDone, fail NotDone, out.line (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { World } from "./world.ts";

/** Every error name a guard in todo.board can give. */
export type Refusal = "NoTask" | "AlreadyDone" | "NotDone";

/** `(world, args) -> (world', lines, reply) | error` (docs/contract.md section 1). */
export type Outcome<R = undefined> =
  | { readonly ok: true; readonly world: World; readonly lines: readonly string[]; readonly reply: R }
  | { readonly ok: false; readonly refusal: Refusal };

export function commit<R>(world: World, lines: readonly string[], reply: R): Outcome<R> {
  return { ok: true, world, lines, reply };
}

export function refuse(refusal: Refusal): Outcome<never> {
  return { ok: false, refusal };
}
