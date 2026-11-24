// Projected by hand from state tasks, state next_no under @store(file(".todo.json")), "Bytes written" (src/todo/board.slg, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

import type { World } from "../board/world.ts";
import type { Tag } from "../model/tag.ts";
import { makeTask, type Task } from "../model/task.ts";
import { asTaskNo, type TaskNo } from "../model/task-no.ts";
import { asTitle } from "../model/title.ts";
import { MAX_NEXT_NO } from "./limits.ts";

/**
 * The bytes the canonical writer produces, recognised in one regex step per task. Contract section 3 fixes
 * those bytes and this program writes them, so a store it wrote is read here without building a JSON tree.
 * Anything this does not recognise, including every invalid store, returns undefined and goes to the
 * general reader, which is the one that decides what is unreadable.
 */
const HEAD = /^\{"version":1,"next_no":([1-9][0-9]{0,15}),"tasks":\[/y;
const TASK =
  // biome-ignore lint/suspicious/noControlCharactersInRegex: JSON forbids raw U+0000..U+001F inside a string.
  /\{"no":([1-9][0-9]{0,15}),"title":"((?:[^"\\\x00-\x1f]|\\["\\])*)","status":"(Open|Done)","priority":"(Low|Normal|High)","tags":\[((?:"[a-z0-9][a-z0-9-]{0,31}"(?:,"[a-z0-9][a-z0-9-]{0,31}")*)?)\]\}/y;
const UNESCAPE = /\\(["\\])/g;

export function readCanonical(text: string): World | undefined {
  HEAD.lastIndex = 0;
  const head = HEAD.exec(text);
  const nextNo = head?.[1] === undefined ? undefined : asTaskNo(BigInt(head[1]));
  if (nextNo === undefined || nextNo > MAX_NEXT_NO) return undefined;
  const tasks = new Map<TaskNo, Task>();
  let at = HEAD.lastIndex;
  let last = 0n;
  while (text[at] !== "]") {
    if (tasks.size > 0 && text[at++] !== ",") return undefined;
    TASK.lastIndex = at;
    const m = TASK.exec(text);
    if (m === null) return undefined;
    at = TASK.lastIndex;
    const [, digits = "", raw = "", status, priority, tagList = ""] = m;
    const no = asTaskNo(BigInt(digits));
    // Ascending numbers rule out repeats; the writer sorts by number.
    if (no === undefined || no <= last || no >= nextNo) return undefined;
    last = no;
    const title = asTitle(raw.includes("\\") ? raw.replace(UNESCAPE, "$1") : raw);
    if (title === undefined || (status !== "Open" && status !== "Done")) return undefined;
    if (priority !== "Low" && priority !== "Normal" && priority !== "High") return undefined;
    const tags = tagList === "" ? [] : (tagList.slice(1, -1).split('","') as Tag[]);
    // Strictly ascending tags rule out repeats; the writer sorts them.
    if (tags.some((t, i) => i > 0 && t <= (tags[i - 1] as Tag))) return undefined;
    tasks.set(no, makeTask(no, title, { status, priority, tags: Object.freeze(tags) }));
  }
  return text.slice(at) === "]}\n" ? { tasks, nextNo } : undefined;
}
