// Projected by hand from states stage, ticks, player, taken, crawlers, lives, mode; View, view (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Crawler, Level } from "../model/level.ts";
import type { Mode, Player } from "../model/player.ts";

/**
 * The seven states. A rule returns a new World; nothing is changed in place.
 * `crawlers` is the `Map[Int, Crawler]` kept as a list in ascending `no`, so `crawlers.values` is the list itself.
 * `lives` is `0..=3`.
 */
export type World = {
  readonly stage: Level;
  readonly ticks: number;
  readonly player: Player;
  readonly taken: ReadonlySet<number>;
  readonly crawlers: readonly Crawler[];
  readonly lives: number;
  readonly mode: Mode;
};

/** Everything the screen needs, read once per frame. Field for field the world, so `view()` is the identity. */
export type View = World;

export const view = (w: World): View => w;
