// Projected by hand from body_half, crawler_half, gem_half, goal_half, run_speed, diagonal_speed, ground_accel, air_accel, gravity, max_fall, jump_speed, hop_cut, coyote_ticks, stomp_bounce, stomp_line, kill_y, Axis, Fall, Placed, body, crawler_box, mover_box, mover_shift, axis, walk_target, facing, slide_x, slide_z, fall, patrol, stomps (src/hop/physics.slg). Stand-in for spec-lang build output; change the spec, not this file.
//! How bodies move in one tick. Pure: every function takes what it reads.

use crate::geom::{self, Box, Vec3};
use crate::model::{Crawler, Mover, Pad, Solid};

/// Half the width, the full height and half the depth of the hero.
pub const BODY_HALF: Vec3 = Vec3::new(64, 192, 64);
pub const CRAWLER_HALF: Vec3 = Vec3::new(96, 128, 96);
pub const GEM_HALF: i32 = 64;
pub const GOAL_HALF: Vec3 = Vec3::new(64, 768, 64);

pub const RUN_SPEED: i32 = 16;
pub const DIAGONAL_SPEED: i32 = 11;
pub const GROUND_ACCEL: i32 = 2;
pub const AIR_ACCEL: i32 = 1;
pub const GRAVITY: i32 = 2;
pub const MAX_FALL: i32 = 48;
pub const JUMP_SPEED: i32 = 40;
pub const HOP_CUT: i32 = 16;
pub const COYOTE_TICKS: i32 = 6;
pub const STOMP_BOUNCE: i32 = 24;
pub const STOMP_LINE: i32 = 64;
pub const KILL_Y: i32 = -2048;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Axis {
    pub pos: i32,
    pub vel: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fall {
    pub y: i32,
    pub vel: i32,
    pub grounded: bool,
    pub riding: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placed {
    pub no: i32,
    pub r#box: Box,
}

pub const fn body(at: Vec3) -> Box {
    geom::around_feet(at, BODY_HALF)
}

pub const fn crawler_box(c: &Crawler) -> Box {
    geom::around_feet(c.at, CRAWLER_HALF)
}

pub const fn mover_box(m: &Mover, t: i32) -> Box {
    geom::shifted(m.home, geom::scaled(m.step, geom::triangle(t, m.span)))
}

/// A rider moves exactly as far as its mover did this tick.
pub const fn mover_shift(m: &Mover, t: i32) -> Vec3 {
    geom::minus(mover_box(m, t).lo, mover_box(m, t - 1).lo)
}

const fn axis(neg: bool, pos: bool) -> i32 {
    pos as i32 - neg as i32
}

/// x runs right, z runs toward the camera, so forward is minus z.
pub const fn walk_target(pad: Pad) -> Vec3 {
    let dx = axis(pad.left, pad.right);
    let dz = axis(pad.forward, pad.back);
    let speed = if dx != 0 && dz != 0 {
        DIAGONAL_SPEED
    } else {
        RUN_SPEED
    };
    Vec3::new(dx * speed, 0, dz * speed)
}

/// The hero faces where the pad points, and keeps facing there when it lets go.
pub const fn facing(pad: Pad, was: i32) -> i32 {
    match (axis(pad.left, pad.right), axis(pad.forward, pad.back)) {
        (1, -1) => 45,
        (1, 1) => 315,
        (1, _) => 0,
        (-1, -1) => 135,
        (-1, 1) => 225,
        (-1, _) => 180,
        (_, -1) => 90,
        (_, 1) => 270,
        _ => was,
    }
}

/// Move on one axis, then stop flush against the first solid in the way.
/// `hits.map(...).min()` is folded as the solids are scanned: no list is built (the filter is pure and only its extreme is read).
pub fn slide_x(at: Vec3, v: i32, solids: &[Solid]) -> Axis {
    if v == 0 {
        return Axis { pos: at.x, vel: 0 };
    }
    let moved = body(at.with_x(at.x + v));
    let mut hits = solids.iter().filter(|s| geom::overlaps(moved, s.r#box));
    let Some(first) = hits.next() else {
        return Axis {
            pos: at.x + v,
            vel: v,
        };
    };
    let pos = if v > 0 {
        hits.fold(first.r#box.lo.x, |m, s| m.min(s.r#box.lo.x)) - BODY_HALF.x
    } else {
        hits.fold(first.r#box.hi.x, |m, s| m.max(s.r#box.hi.x)) + BODY_HALF.x
    };
    Axis { pos, vel: 0 }
}

pub fn slide_z(at: Vec3, v: i32, solids: &[Solid]) -> Axis {
    if v == 0 {
        return Axis { pos: at.z, vel: 0 };
    }
    let moved = body(at.with_z(at.z + v));
    let mut hits = solids.iter().filter(|s| geom::overlaps(moved, s.r#box));
    let Some(first) = hits.next() else {
        return Axis {
            pos: at.z + v,
            vel: v,
        };
    };
    let pos = if v > 0 {
        hits.fold(first.r#box.lo.z, |m, s| m.min(s.r#box.lo.z)) - BODY_HALF.z
    } else {
        hits.fold(first.r#box.hi.z, |m, s| m.max(s.r#box.hi.z)) + BODY_HALF.z
    };
    Axis { pos, vel: 0 }
}

/// Solids stop the body both ways; a mover's top only catches feet that were above it.
pub fn fall(at: Vec3, v: i32, solids: &[Solid], tops: &[Placed]) -> Fall {
    let moved = body(at.with_y(at.y + v));
    let mut hits = solids.iter().filter(|s| geom::overlaps(moved, s.r#box));
    if let Some(first) = hits.next() {
        return if v > 0 {
            Fall {
                y: hits.fold(first.r#box.lo.y, |m, s| m.min(s.r#box.lo.y)) - BODY_HALF.y,
                vel: 0,
                grounded: false,
                riding: None,
            }
        } else {
            Fall {
                y: hits.fold(first.r#box.hi.y, |m, s| m.max(s.r#box.hi.y)),
                vel: 0,
                grounded: true,
                riding: None,
            }
        };
    }
    let here = body(at);
    // One pass keeps the highest catching top and, among tops at that height, the lowest number:
    // `catching.map(hi.y).max()` then `filter(hi.y == top).map(no).min()`.
    let caught = tops
        .iter()
        .filter(|p| {
            v < 0 && geom::over(here, p.r#box) && at.y >= p.r#box.hi.y && at.y + v < p.r#box.hi.y
        })
        .fold(None::<(i32, i32)>, |best, p| {
            let candidate = (p.r#box.hi.y, p.no);
            match best {
                Some((top, no))
                    if top > candidate.0 || (top == candidate.0 && no <= candidate.1) =>
                {
                    best
                }
                _ => Some(candidate),
            }
        });
    match caught {
        None => Fall {
            y: at.y + v,
            vel: v,
            grounded: false,
            riding: None,
        },
        Some((top, no)) => Fall {
            y: top,
            vel: 0,
            grounded: true,
            riding: Some(no),
        },
    }
}

/// A crawler walks to the end of its beat and turns.
pub const fn patrol(c: Crawler) -> Crawler {
    let x = c.at.x + c.dir * c.speed;
    if x >= c.hi {
        Crawler {
            at: c.at.with_x(c.hi),
            dir: -1,
            ..c
        }
    } else if x <= c.lo {
        Crawler {
            at: c.at.with_x(c.lo),
            dir: 1,
            ..c
        }
    } else {
        Crawler {
            at: c.at.with_x(x),
            ..c
        }
    }
}

/// From above and falling, on the upper half: a stomp; any other touch hurts.
pub const fn stomps(feet: i32, vy: i32, c: &Crawler) -> bool {
    vy < 0 && feet >= c.at.y + STOMP_LINE
}
