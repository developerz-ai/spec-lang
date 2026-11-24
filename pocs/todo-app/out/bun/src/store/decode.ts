// Projected by hand from state tasks, state next_no, Task under @store(file(".todo.json")) (src/todo/board.slg, src/todo/model.slg, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

import type { World } from "../board/world.ts";
import { PRIORITIES, type Priority } from "../model/priority.ts";
import { STATUSES, type Status } from "../model/status.ts";
import { asTag, type Tag, tagSet } from "../model/tag.ts";
import { makeTask, type Task } from "../model/task.ts";
import { asTaskNo, type TaskNo } from "../model/task-no.ts";
import { asTitle } from "../model/title.ts";
import type { Json } from "./json-reader.ts";
import { MAX_NEXT_NO } from "./limits.ts";

export type Decoded = { readonly ok: true; readonly world: World } | { readonly ok: false; readonly reason: string };

class Unfit extends Error {}

function unfit(why: string): never {
  throw new Unfit(why);
}

const WORLD_KEYS = Object.freeze(["version", "next_no", "tasks"]);
const TASK_KEYS = Object.freeze(["no", "title", "status", "priority", "tags"]);

/** Contract section 3, "Accepted on read". Order of keys and of tasks is free; every field is checked. */
export function decodeWorld(json: Json): Decoded {
  try {
    const top = fields(json, WORLD_KEYS, "store");
    if (integer(top.get("version"), "version") !== 1n) unfit("version is not 1");
    const nextNo = asTaskNo(integer(top.get("next_no"), "next_no"));
    if (nextNo === undefined || nextNo > MAX_NEXT_NO) unfit("next_no out of range");
    const tasks = new Map<TaskNo, Task>();
    for (const item of array(top.get("tasks"), "tasks")) {
      const t = task(item, nextNo);
      if (tasks.has(t.no)) unfit(`task ${t.no} twice`);
      tasks.set(t.no, t);
    }
    return { ok: true, world: { tasks, nextNo } };
  } catch (e) {
    if (e instanceof Unfit) return { ok: false, reason: e.message };
    throw e;
  }
}

function task(json: Json, nextNo: TaskNo): Task {
  const f = fields(json, TASK_KEYS, "task");
  const no = asTaskNo(integer(f.get("no"), "no"));
  if (no === undefined || no >= nextNo) unfit("no out of range");
  const title = asTitle(string(f.get("title"), "title"));
  if (title === undefined) unfit("title is not a Title");
  const status = oneOf<Status>(STATUSES, string(f.get("status"), "status"), "status");
  const priority = oneOf<Priority>(PRIORITIES, string(f.get("priority"), "priority"), "priority");
  const tags: Tag[] = [];
  let ascending = true;
  for (const item of array(f.get("tags"), "tags")) {
    const tag = asTag(string(item, "tag"));
    if (tag === undefined) unfit("tag is not a Tag");
    const last = tags[tags.length - 1];
    // The canonical writer sorts tags, so strictly ascending is the common case: no repeat is then possible.
    if (!ascending || (last !== undefined && tag <= last)) {
      ascending = false;
      if (tags.includes(tag)) unfit("tag twice");
    }
    tags.push(tag);
  }
  return makeTask(no, title, { status, priority, tags: ascending ? Object.freeze(tags) : tagSet(tags) });
}

function fields(json: Json, keys: readonly string[], what: string): ReadonlyMap<string, Json> {
  if (json.kind !== "object") unfit(`${what} is not an object`);
  if (json.entries.size !== keys.length || !keys.every((k) => json.entries.has(k))) unfit(`${what} keys`);
  return json.entries;
}

function integer(json: Json | undefined, what: string): bigint {
  if (json?.kind !== "number" || json.integer === undefined) unfit(`${what} is not an integer`);
  return json.integer;
}

function string(json: Json | undefined, what: string): string {
  if (json?.kind !== "string") unfit(`${what} is not a string`);
  return json.value;
}

function array(json: Json | undefined, what: string): readonly Json[] {
  if (json?.kind !== "array") unfit(`${what} is not an array`);
  return json.items;
}

function oneOf<T extends string>(names: readonly T[], s: string, what: string): T {
  const found = names.find((n) => n === s);
  if (found === undefined) unfit(`${what} is not one of ${names.join(", ")}`);
  return found;
}
