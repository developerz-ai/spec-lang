// Projected by hand from Status (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

export const STATUSES = Object.freeze(["Open", "Done"] as const);
export type Status = (typeof STATUSES)[number];
