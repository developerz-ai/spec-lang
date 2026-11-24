// Projected by hand from state tasks, state next_no under @store(file(".todo.json")), "Bytes written" (src/todo/board.slg, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

import type { World } from "../board/world.ts";
import type { Task } from "../model/task.ts";
import { MAX_NEXT_NO } from "./limits.ts";

export type Encoded = { readonly ok: true; readonly text: string } | { readonly ok: false; readonly error: "full" };

const ESCAPE = /["\\]/g;

/** Only `"` and `\` are escaped; Title and Tag claims rule out every character JSON would also require escaping. */
function quote(s: string): string {
  return `"${s.replace(ESCAPE, (c) => `\\${c}`)}"`;
}

function taskJson(t: Task): string {
  return `{"no":${t.no},"title":${quote(t.title)},"status":"${t.status}","priority":"${t.priority}","tags":[${t.tags.map(quote).join(",")}]}`;
}

/** The canonical bytes: one line, keys in fixed order, tasks by `no` ascending, tags in code-point order. */
export function encodeWorld(w: World): Encoded {
  if (w.nextNo > MAX_NEXT_NO) return { ok: false, error: "full" };
  const tasks = [...w.tasks.values()].sort((a, b) => (a.no < b.no ? -1 : 1)).map(taskJson);
  return { ok: true, text: `{"version":1,"next_no":${w.nextNo},"tasks":[${tasks.join(",")}]}\n` };
}
