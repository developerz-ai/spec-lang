// Projected by hand from rule stats (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { commit, type Outcome } from "./outcome.ts";
import type { World } from "./world.ts";

/** Reads only: the world comes back unchanged. */
export function stats(w: World): Outcome {
  let open = 0;
  for (const t of w.tasks.values()) if (t.status === "Open") open++;
  return commit(w, [`${open} open, ${w.tasks.size - open} done`], undefined);
}
