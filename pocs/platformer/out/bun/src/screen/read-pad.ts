// Projected by hand from key_down, the key constants, read_pad (src/hop/screen.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Pad } from "../model/pad.ts";
import type { Raylib } from "./raylib.ts";

const KEY_RIGHT = 262;
const KEY_LEFT = 263;
const KEY_DOWN_ARROW = 264;
const KEY_UP = 265;
const KEY_SPACE = 32;
const KEY_A = 65;
const KEY_D = 68;
const KEY_S = 83;
const KEY_W = 87;

/** Arrows or WASD to run, space to jump. */
export function readPad(rl: Raylib): Pad {
  const keyDown = rl.c.IsKeyDown;
  return {
    left: keyDown(KEY_LEFT) || keyDown(KEY_A),
    right: keyDown(KEY_RIGHT) || keyDown(KEY_D),
    forward: keyDown(KEY_UP) || keyDown(KEY_W),
    back: keyDown(KEY_DOWN_ARROW) || keyDown(KEY_S),
    jump: keyDown(KEY_SPACE),
  };
}
