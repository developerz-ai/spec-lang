// Projected by hand from the commands play, run, draw, shot of app SkyhopBun (docs/game-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import type { LevelNo } from "../levels/level.ts";

/** `at` is exact at any length (`null`: the end of the script); `out` is `null` when not given. */
export type Command =
  | { readonly name: "play"; readonly level: LevelNo }
  | { readonly name: "run"; readonly level: LevelNo }
  | { readonly name: "draw"; readonly level: LevelNo; readonly at: bigint | null }
  | { readonly name: "shot"; readonly level: LevelNo; readonly at: bigint | null; readonly out: string | null };

export type CommandName = Command["name"];

export type Flag = "level" | "at" | "out";

/** `--level` from the parameter of `start`; `--at` and `--out` from the vocabulary's `draw` and `shot`. */
export const flagsOf: Record<CommandName, readonly Flag[]> = {
  play: ["level"],
  run: ["level"],
  draw: ["level", "at"],
  shot: ["level", "at", "out"],
};
