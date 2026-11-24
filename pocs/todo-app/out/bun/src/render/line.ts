// Projected by hand from line (src/todo/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Task } from "../model/task.ts";
import { badge } from "./badge.ts";
import { mark } from "./mark.ts";
import { tagText } from "./tag-text.ts";

/** `"{t.no} [{mark(t.status)}] {t.title}{badge(t.priority)}{tag_text(t.tags)}"` */
export function line(t: Task): string {
  return `${t.no} [${mark(t.status)}] ${t.title}${badge(t.priority)}${tagText(t.tags)}`;
}
