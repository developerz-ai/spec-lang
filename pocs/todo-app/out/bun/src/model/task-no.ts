// Projected by hand from TaskNo (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

declare const taskNoBrand: unique symbol;

/** `Int where self >= 1`. `Int` is exact and unbounded, so a bigint. */
export type TaskNo = bigint & { readonly [taskNoBrand]: true };

export function asTaskNo(n: bigint): TaskNo | undefined {
  return n >= 1n ? (n as TaskNo) : undefined;
}

export const FIRST_NO = 1n as TaskNo;

export function nextNo(n: TaskNo): TaskNo {
  return (n + 1n) as TaskNo;
}
