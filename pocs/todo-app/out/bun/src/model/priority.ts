// Projected by hand from Priority (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

export const PRIORITIES = Object.freeze(["Low", "Normal", "High"] as const);
export type Priority = (typeof PRIORITIES)[number];
