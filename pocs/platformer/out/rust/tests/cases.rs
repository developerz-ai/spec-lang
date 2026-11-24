// Projected by hand from the @cases of plus, around_feet, overlaps, touches, triangle, approach, vec_text (src/hop/geom.slg), q (src/hop/levels.slg), walk_target, facing, patrol (src/hop/physics.slg), asset_name, bob, draw_text (src/hop/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

use skyhop::geom::{self, Box, Vec3};
use skyhop::levels;
use skyhop::model::{Crawler, Pad, Rgb};
use skyhop::physics;
use skyhop::render::{self, Asset, Draw};

const fn v(x: i32, y: i32, z: i32) -> Vec3 {
    Vec3::new(x, y, z)
}

const fn b(lo: Vec3, hi: Vec3) -> Box {
    Box::new(lo, hi)
}

#[test]
fn plus_cases() {
    assert_eq!(geom::plus(v(1, 2, 3), v(10, 20, 30)), v(11, 22, 33));
}

#[test]
fn around_feet_cases() {
    assert_eq!(
        geom::around_feet(v(0, 0, 0), v(64, 192, 64)),
        b(v(-64, 0, -64), v(64, 192, 64))
    );
}

#[test]
fn overlaps_cases() {
    let cases = [
        (b(v(0, 0, 0), v(2, 2, 2)), b(v(1, 1, 1), v(3, 3, 3)), true),
        (b(v(0, 0, 0), v(2, 2, 2)), b(v(2, 0, 0), v(4, 2, 2)), false),
    ];
    for (x, y, want) in cases {
        assert_eq!(geom::overlaps(x, y), want, "overlaps({x:?}, {y:?})");
    }
}

#[test]
fn touches_cases() {
    let cases = [
        (b(v(0, 0, 0), v(2, 2, 2)), b(v(2, 0, 0), v(4, 2, 2)), true),
        (b(v(0, 0, 0), v(2, 2, 2)), b(v(3, 0, 0), v(4, 2, 2)), false),
    ];
    for (x, y, want) in cases {
        assert_eq!(geom::touches(x, y), want, "touches({x:?}, {y:?})");
    }
}

#[test]
fn triangle_cases() {
    for (t, n, want) in [
        (0, 4, 0),
        (3, 4, 3),
        (4, 4, 4),
        (5, 4, 3),
        (8, 4, 0),
        (9, 4, 1),
    ] {
        assert_eq!(geom::triangle(t, n), want, "triangle({t}, {n})");
    }
}

#[test]
fn approach_cases() {
    for (x, target, step, want) in [(0, 3, 2, 2), (5, 3, 2, 3), (5, 0, 2, 3), (-5, 0, 1, -4)] {
        assert_eq!(
            geom::approach(x, target, step),
            want,
            "approach({x}, {target}, {step})"
        );
    }
}

#[test]
fn vec_text_cases() {
    assert_eq!(geom::vec_text(v(1, -2, 3)), "1,-2,3");
}

#[test]
fn q_cases() {
    assert_eq!(levels::q(4), 256);
    assert_eq!(levels::q(-2), -128);
}

fn pad(left: bool, right: bool, forward: bool, back: bool) -> Pad {
    Pad {
        left,
        right,
        forward,
        back,
        jump: false,
    }
}

#[test]
fn walk_target_cases() {
    let cases = [
        (Pad::default(), v(0, 0, 0)),
        (pad(false, true, false, false), v(16, 0, 0)),
        (pad(true, false, true, false), v(-11, 0, -11)),
        (pad(true, true, false, false), v(0, 0, 0)),
    ];
    for (p, want) in cases {
        assert_eq!(physics::walk_target(p), want, "walk_target({p:?})");
    }
}

#[test]
fn facing_cases() {
    let cases = [
        (Pad::default(), 90, 90),
        (pad(false, true, false, false), 90, 0),
        (pad(false, false, true, false), 0, 90),
        (pad(true, false, false, true), 0, 225),
    ];
    for (p, was, want) in cases {
        assert_eq!(physics::facing(p, was), want, "facing({p:?}, {was})");
    }
}

#[test]
fn patrol_cases() {
    let c = |x, dir| Crawler {
        no: 1,
        at: v(x, 0, 0),
        lo: -10,
        hi: 10,
        dir,
        speed: 4,
    };
    assert_eq!(physics::patrol(c(0, 1)), c(4, 1));
    assert_eq!(physics::patrol(c(8, 1)), c(10, -1));
}

#[test]
fn asset_name_cases() {
    let cases = [
        (Asset::PipMesh, "pip"),
        (Asset::CrawlerMesh, "crawler"),
        (Asset::GemMesh, "gem"),
        (Asset::FlagMesh, "flag"),
    ];
    for (a, want) in cases {
        assert_eq!(render::asset_name(a), want);
    }
}

#[test]
fn bob_cases() {
    for (t, want) in [(0, -32), (16, 0), (32, 32), (48, 0)] {
        assert_eq!(render::bob(t), want, "bob({t})");
    }
}

#[test]
fn draw_text_cases() {
    let cube = Draw::Cube {
        r#box: b(v(0, 0, 0), v(64, 64, 64)),
        color: Rgb::new(1, 2, 3),
    };
    assert_eq!(render::draw_text(&cube), "cube 0,0,0 64,64,64 1,2,3");
    let mesh = Draw::Mesh {
        asset: Asset::PipMesh,
        at: v(1, 2, 3),
        yaw: 90,
    };
    assert_eq!(render::draw_text(&mesh), "mesh pip 1,2,3 90");
}
