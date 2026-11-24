import type { ArgType } from "./arg-types.ts";

/**
 * How a rule parameter appears on the command line:
 * - positional: a parameter without a default, `<name>`
 * - switch: `Bool = false`, `--name`
 * - once: `Option[T] = none` or a defaulted sum, `--name <value>` at most once
 * - repeat: `Set[T]` with a default, `--name <value>` repeatable, repeats collapse
 */
export type Shape = "positional" | "switch" | "once" | "repeat";

export const MISSING: unique symbol = Symbol("missing");

export interface Param<T> {
  readonly name: string;
  readonly shape: Shape;
  /** Absent for a switch, which takes no value. */
  readonly type: ArgType<unknown> | undefined;
  /** The value when the argument is not given; MISSING for a positional. */
  readonly fallback: T | typeof MISSING;
  /** This parameter's fragment of the command's `--help` line. */
  readonly usage: string;
}

export function positional<T>(name: string, type: ArgType<T>): Param<T> {
  return Object.freeze({ name, shape: "positional", type, fallback: MISSING, usage: `<${name}>` });
}

export function switchFlag(name: string): Param<boolean> {
  return Object.freeze({ name, shape: "switch", type: undefined, fallback: false, usage: `[--${name}]` });
}

export function option<T>(name: string, type: ArgType<T>): Param<T | undefined> {
  return Object.freeze({ name, shape: "once", type, fallback: undefined, usage: `[--${name} ${type.hint}]` });
}

export function defaulted<T>(name: string, type: ArgType<T>, fallback: T): Param<T> {
  return Object.freeze({ name, shape: "once", type, fallback, usage: `[--${name} ${type.hint}]` });
}

export function repeated<T>(name: string, type: ArgType<T>): Param<ReadonlySet<T>> {
  const fallback: ReadonlySet<T> = new Set<T>();
  return Object.freeze({ name, shape: "repeat", type, fallback, usage: `[--${name} ${type.hint}]...` });
}

/** The values of a parameter list, in order, typed per parameter. */
export type Values<P extends readonly Param<unknown>[]> = {
  readonly [K in keyof P]: P[K] extends Param<infer T> ? T : never;
};
