// Projected by hand from has_tag, rule list (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Tag } from "../model/tag.ts";
import type { Task } from "../model/task.ts";
import { line } from "../render/line.ts";
import { compareRank } from "../render/rank.ts";
import { commit, type Outcome } from "./outcome.ts";
import type { World } from "./world.ts";

export function hasTag(t: Task, tag: Tag | undefined): boolean {
  return tag === undefined || t.tags.includes(tag);
}

/** Open work first; finished work only when asked for. Reads only: the world comes back unchanged. */
export function list(w: World, all = false, tag: Tag | undefined = undefined): Outcome {
  const shown = [...w.tasks.values()].filter((t) => (all || t.status === "Open") && hasTag(t, tag)).sort(compareRank);
  if (shown.length === 0) return commit(w, ["nothing to do"], undefined);
  return commit(w, shown.map(line), undefined);
}
