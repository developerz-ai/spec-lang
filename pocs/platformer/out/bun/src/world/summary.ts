// Projected by hand from mode_text, summary (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { vecText } from "../geom/vec3.ts";
import type { Mode } from "../model/player.ts";
import type { World } from "./world.ts";

const modeText: Record<Mode, string> = { Playing: "playing", Won: "won", Lost: "lost" };

/** One line that sums up a run, printed after the last tick. */
export const summary = (w: World): string =>
  `end ${w.ticks} ${modeText[w.mode]} gems ${w.taken.size}/${w.stage.gems.length} lives ${w.lives} at ${vecText(w.player.at)}`;
