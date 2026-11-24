// Projected by hand from rule start (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { type LevelNo, level as levelOf } from "../levels/level.ts";
import { player } from "../model/player.ts";
import type { World } from "./world.ts";

/** Every level starts the same way: three lives, nothing taken, every crawler home. It has no guard, so it cannot refuse. */
export function start(level: LevelNo): { readonly world: World; readonly lines: readonly string[] } {
  const l = levelOf(level);
  return {
    world: {
      stage: l,
      ticks: 0,
      player: player(l.start),
      taken: new Set(),
      // `crawlers[c.no] = c` for each, read back by ascending key.
      crawlers: [...new Map(l.crawlers.map((c) => [c.no, c])).values()].sort((a, b) => a.no - b.no),
      lives: 3,
      mode: "Playing",
    },
    lines: [`0 level ${l.name}`],
  };
}
