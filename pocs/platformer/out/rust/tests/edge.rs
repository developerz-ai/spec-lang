// Projected by hand from test touch_matches_raylib (src/hop/tests/edge.slg) and check_collision_boxes (src/hop/screen.slg). Stand-in for spec-lang build output; change the spec, not this file.
//! The game's touch against raylib's `CheckCollisionBoxes`, through the real library. That function is plain arithmetic in
//! raylib and needs no window.

mod support;

use skyhop::geom::{self, Box, Sub, Vec3};
use skyhop::screen;
use support::Rng;

const SUB: i64 = 1_048_576;
const PAIRS: u64 = 4000;

fn sub(rng: &mut Rng) -> Sub {
    rng.int(-SUB, SUB) as Sub
}

/// Any box: on each axis two `Sub` values, the smaller as `lo`.
fn any_box(rng: &mut Rng) -> Box {
    let mut axis = || {
        let (a, b) = (sub(rng), sub(rng));
        (a.min(b), a.max(b))
    };
    let (x, y, z) = (axis(), axis(), axis());
    Box::new(Vec3::new(x.0, y.0, z.0), Vec3::new(x.1, y.1, z.1))
}

/// A box that meets `a` face to face, or misses or overlaps it by one sub-metre, on one random axis; small elsewhere.
fn near_box(a: Box, rng: &mut Rng) -> Box {
    let size = |rng: &mut Rng| rng.int(0, 4096) as Sub;
    let gap = rng.int(-1, 1) as Sub;
    let clamp = |v: Sub| v.clamp(-(SUB as Sub), SUB as Sub);
    let place = |lo: Sub, hi: Sub, rng: &mut Rng| {
        let at = clamp(rng.int(i64::from(lo) - 64, i64::from(hi) + 64) as Sub);
        (at, clamp(at + size(rng)))
    };
    let (mut x, mut y, mut z) = (
        place(a.lo.x, a.hi.x, rng),
        place(a.lo.y, a.hi.y, rng),
        place(a.lo.z, a.hi.z, rng),
    );
    let after = |hi: Sub, rng: &mut Rng| {
        let lo = clamp(hi + gap);
        (lo, clamp(lo + size(rng)))
    };
    let before = |lo: Sub, rng: &mut Rng| {
        let hi = clamp(lo - gap);
        (clamp(hi - size(rng)), hi)
    };
    let side = rng.bool();
    match rng.int(0, 2) {
        0 => {
            x = if side {
                after(a.hi.x, rng)
            } else {
                before(a.lo.x, rng)
            }
        }
        1 => {
            y = if side {
                after(a.hi.y, rng)
            } else {
                before(a.lo.y, rng)
            }
        }
        _ => {
            z = if side {
                after(a.hi.z, rng)
            } else {
                before(a.lo.z, rng)
            }
        }
    }
    Box::new(Vec3::new(x.0, y.0, z.0), Vec3::new(x.1, y.1, z.1))
}

#[test]
fn touch_matches_raylib() {
    let mut rng = Rng::new(0xed9e);
    let (mut touching, mut apart) = (0, 0);
    for i in 0..PAIRS {
        let a = any_box(&mut rng);
        let b = if i % 2 == 0 {
            near_box(a, &mut rng)
        } else {
            any_box(&mut rng)
        };
        let ours = geom::touches(a, b);
        assert_eq!(
            ours,
            screen::check_collision_boxes(a, b),
            "pair {i}: {a:?} {b:?}"
        );
        if ours { touching += 1 } else { apart += 1 }
    }
    assert!(
        touching > 400 && apart > 400,
        "{touching} touching, {apart} apart"
    );
}
