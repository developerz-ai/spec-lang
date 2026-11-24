// Projected by hand from key_down, key_right, key_left, key_down_arrow, key_up, key_space, key_a, key_d, key_s, key_w, read_pad (src/hop/screen.slg). Stand-in for spec-lang build output; change the spec, not this file.

use super::ffi;
use crate::model::Pad;

const KEY_RIGHT: i32 = 262;
const KEY_LEFT: i32 = 263;
const KEY_DOWN_ARROW: i32 = 264;
const KEY_UP: i32 = 265;
const KEY_SPACE: i32 = 32;
const KEY_A: i32 = 65;
const KEY_D: i32 = 68;
const KEY_S: i32 = 83;
const KEY_W: i32 = 87;

fn key_down(key: i32) -> bool {
    // SAFETY: reads raylib's keyboard state; called only while a window is open (`Window::play`).
    unsafe { ffi::IsKeyDown(key) }
}

/// Arrows or WASD to run, space to jump.
pub fn read_pad() -> Pad {
    Pad {
        left: key_down(KEY_LEFT) || key_down(KEY_A),
        right: key_down(KEY_RIGHT) || key_down(KEY_D),
        forward: key_down(KEY_UP) || key_down(KEY_W),
        back: key_down(KEY_DOWN_ARROW) || key_down(KEY_S),
        jump: key_down(KEY_SPACE),
    }
}
