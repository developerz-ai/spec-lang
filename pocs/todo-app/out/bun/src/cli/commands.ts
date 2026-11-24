// Projected by hand from @on(cli(...)) on add, list, done, reopen, rename, remove, clear, stats (src/todo/board.slg, src/apps/bun.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { add } from "../board/add.ts";
import { clear } from "../board/clear.ts";
import { done, reopen } from "../board/done.ts";
import { list } from "../board/list.ts";
import type { Outcome } from "../board/outcome.ts";
import { remove } from "../board/remove.ts";
import { rename } from "../board/rename.ts";
import { stats } from "../board/stats.ts";
import type { World } from "../board/world.ts";
import { tagSet } from "../model/tag.ts";
import { PRIORITY, TAG, TASK_NO, TITLE } from "./arg-types.ts";
import { defaulted, option, type Param, positional, repeated, switchFlag, type Values } from "./params.ts";

/**
 * From `@uses`: `read` rules use `tasks.read` only, so they take a shared lock and never write the store;
 * `write` rules use `tasks.write` and take an exclusive lock.
 */
export type Access = "read" | "write";

export interface Command {
  readonly name: string;
  readonly access: Access;
  readonly params: readonly Param<unknown>[];
  readonly invoke: (world: World, values: readonly unknown[]) => Outcome<unknown>;
}

function command<const P extends readonly Param<unknown>[]>(
  name: string,
  access: Access,
  params: P,
  rule: (world: World, values: Values<P>) => Outcome<unknown>,
): Command {
  // The parser fills `values` from `params` one for one, each through its own ArgType, so the shapes agree.
  return Object.freeze({
    name,
    access,
    params,
    invoke: (world: World, values: readonly unknown[]) => rule(world, values as Values<P>),
  });
}

/** In the order of `app TodoBun(rules: [...])`, which is also the order of `--help`. */
export const COMMANDS: readonly Command[] = Object.freeze([
  command(
    "add",
    "write",
    [positional("title", TITLE), defaulted("priority", PRIORITY, "Normal"), repeated("tags", TAG)],
    (w, [title, priority, tags]) => add(w, title, priority, tagSet(tags)),
  ),
  command("list", "read", [switchFlag("all"), option("tag", TAG)], (w, [all, tag]) => list(w, all, tag)),
  command("done", "write", [positional("no", TASK_NO)], (w, [no]) => done(w, no)),
  command("reopen", "write", [positional("no", TASK_NO)], (w, [no]) => reopen(w, no)),
  command("rename", "write", [positional("no", TASK_NO), positional("title", TITLE)], (w, [no, title]) =>
    rename(w, no, title),
  ),
  command("remove", "write", [positional("no", TASK_NO)], (w, [no]) => remove(w, no)),
  command("clear", "write", [], (w) => clear(w)),
  command("stats", "read", [], (w) => stats(w)),
]);

export const COMMAND_BY_NAME: ReadonlyMap<string, Command> = new Map(COMMANDS.map((c) => [c.name, c]));
