// Projected by hand from tag_text (src/todo/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { TagSet } from "../model/tag.ts";

/** `tags.sort(it).map(t => " #{t}").join("")`; a TagSet is already sorted. */
export function tagText(tags: TagSet): string {
  let s = "";
  for (const t of tags) s += ` #${t}`;
  return s;
}
