// Projected by hand from play: start = world.start, tick = world.step, input = screen.read_pad, frame = screen.present, report = world.summary (docs/game-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import type { LevelNo } from "../levels/level.ts";
import { windowLoop } from "../screen/loop.ts";
import { start } from "../world/start.ts";
import { step } from "../world/step.ts";
import { summary } from "../world/summary.ts";
import { view, type World } from "../world/world.ts";
import type { Result } from "./result.ts";
import { screenFailure } from "./screen-failure.ts";
import type { Screen } from "./shot.ts";

export type WindowNote = { readonly width: number; readonly height: number; readonly fps: number };

/** One tick per frame at a fixed rate; `write` is flushed by the caller each time, so lines appear as they happen. */
export function play(level: LevelNo, screen: Screen, window: WindowNote, write: (text: string) => void): Result {
  const emit = (lines: readonly string[]): void => {
    if (lines.length > 0) write(lines.map((l) => `${l}\n`).join(""));
  };
  let world: World | undefined;
  const looped = windowLoop(
    { ...screen, width: window.width, height: window.height },
    window.fps,
    () => {
      const began = start(level);
      emit(began.lines);
      world = began.world;
      return view(began.world);
    },
    (pad) => {
      const now = world as World;
      if (now.mode !== "Playing") return view(now);
      const o = step(now, pad());
      if (!o.ok) return view(now);
      emit(o.lines);
      world = o.world;
      return view(o.world);
    },
    () => emit([summary(world as World)]),
  );
  return looped.ok ? { stdout: "", stderr: "", code: 0 } : screenFailure(looped.error);
}
