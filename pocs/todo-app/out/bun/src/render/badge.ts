// Projected by hand from badge (src/todo/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Priority } from "../model/priority.ts";

const BADGES: Readonly<Record<Priority, string>> = Object.freeze({ Low: " !low", Normal: "", High: " !high" });

export function badge(p: Priority): string {
  return BADGES[p];
}
