import type { RuleOutcome } from "../outcome.ts";
import type { Param, Values } from "./params.ts";

/**
 * From the rule's write set: a `read` rule assigns no state, so it takes a shared lock and never
 * writes the store; a `write` rule takes an exclusive lock.
 */
export type Access = "read" | "write";

export interface Command<W> {
  readonly name: string;
  readonly access: Access;
  readonly params: readonly Param<unknown>[];
  readonly invoke: (world: W, values: readonly unknown[]) => RuleOutcome<W, string, unknown>;
}

export function command<W, const P extends readonly Param<unknown>[]>(
  name: string,
  access: Access,
  params: P,
  rule: (world: W, values: Values<P>) => RuleOutcome<W, string, unknown>,
): Command<W> {
  // The parser fills `values` from `params` one for one, each through its own ArgType, so the shapes agree.
  return Object.freeze({
    name,
    access,
    params,
    invoke: (world: W, values: readonly unknown[]) => rule(world, values as Values<P>),
  });
}

/** `--help`, derived from the command table: one line per command, in the app's order. */
export function helpText<W>(bin: string, commands: readonly Command<W>[]): string {
  return [
    `usage: ${bin} <command> [args]`,
    "",
    "commands:",
    ...commands.map((c) => ["  ", c.name, ...c.params.map((p) => ` ${p.usage}`)].join("")),
    "",
  ].join("\n");
}
