// Projected by hand from mark (src/todo/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Status } from "../model/status.ts";

const MARKS: Readonly<Record<Status, string>> = Object.freeze({ Open: " ", Done: "x" });

export function mark(s: Status): string {
  return MARKS[s];
}
