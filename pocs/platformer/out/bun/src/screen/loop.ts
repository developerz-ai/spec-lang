// Projected by hand from the 60 Hz window loop of play: input = screen.read_pad, frame = screen.present (docs/game-vocabulary.md, docs/contract.md section 2 "play"). Stand-in for spec-lang build output; change the spec, not this file.

import type { Pad } from "../model/pad.ts";
import type { View } from "../world/world.ts";
import { present } from "./present.ts";
import { readPad } from "./read-pad.ts";
import { closeWindow, FLAG_MSAA_4X_HINT, openWindow, type ScreenError, type WindowSpec } from "./window.ts";

export type Looped<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly error: ScreenError };

/**
 * Opens the window, then until Esc or the close button: `frame(pad)` gives the view to draw.
 * `begin` runs once after the window and its assets are ready, `end` after the last frame.
 * The pad is read lazily: a frame that does not tick does not read the keys.
 */
export function windowLoop<T>(
  spec: Omit<WindowSpec, "flags">,
  fps: number,
  begin: () => View,
  frame: (pad: () => Pad) => View,
  end: () => T,
): Looped<T> {
  const opened = openWindow({ ...spec, flags: FLAG_MSAA_4X_HINT });
  if (!opened.ok) return opened;
  const { rl, canvas } = opened.window;
  rl.c.SetTargetFPS(fps);
  const pad = (): Pad => readPad(rl);
  begin();
  while (!rl.c.WindowShouldClose()) {
    const view = frame(pad);
    rl.c.BeginDrawing();
    present(canvas, view);
    rl.c.EndDrawing();
  }
  const value = end();
  closeWindow(opened.window);
  return { ok: true, value };
}
