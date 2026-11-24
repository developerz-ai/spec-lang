// Projected by hand from LevelNo, grass, stone, wood, q, at, slab, meadow, ridge, level (src/hop/levels.slg). Stand-in for spec-lang build output; change the spec, not this file.
//! The two levels, as `const` data computed by the compiler.

use crate::geom::{Box, Vec3};
use crate::model::{Crawler, Gem, Level, Mover, Rgb, Solid};

/// `type LevelNo = 1..=2`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LevelNo {
    One,
    Two,
}

impl LevelNo {
    pub const fn new(n: u64) -> Option<Self> {
        match n {
            1 => Some(Self::One),
            2 => Some(Self::Two),
            _ => None,
        }
    }
}

const GRASS: Rgb = Rgb::new(98, 170, 80);
const STONE: Rgb = Rgb::new(150, 150, 160);
const WOOD: Rgb = Rgb::new(210, 150, 60);

/// Levels are laid out on a quarter-metre grid: 64 sub-metres.
pub const fn q(n: i32) -> i32 {
    n * 64
}

const fn at(x: i32, y: i32, z: i32) -> Vec3 {
    Vec3::new(q(x), q(y), q(z))
}

const fn slab(from: Vec3, to: Vec3, color: Rgb) -> Solid {
    Solid {
        r#box: Box::new(from, to),
        color,
    }
}

/// Teaches running, a gap, a step up, a crawler and the flag.
pub const MEADOW: Level = Level {
    name: "meadow",
    start: at(0, 0, 0),
    solids: &[
        slab(at(-8, -4, -8), at(16, 0, 8), GRASS),
        slab(at(24, -4, -8), at(40, 0, 8), GRASS),
        slab(at(40, -4, -8), at(56, 4, 8), STONE),
        slab(at(64, -4, -8), at(84, 0, 8), GRASS),
    ],
    movers: &[],
    gems: &[
        Gem {
            no: 1,
            at: at(32, 6, 0),
        },
        Gem {
            no: 2,
            at: at(60, 10, 0),
        },
        Gem {
            no: 3,
            at: at(72, 2, 0),
        },
    ],
    crawlers: &[Crawler {
        no: 1,
        at: at(48, 4, 0),
        lo: q(43),
        hi: q(53),
        dir: 1,
        speed: 2,
    }],
    goal: at(80, 0, 0),
};

/// A moving platform over a wide gap, then two crawlers on high ground a jump can reach.
pub const RIDGE: Level = Level {
    name: "ridge",
    start: at(0, 0, 0),
    solids: &[
        slab(at(-8, -4, -8), at(16, 0, 8), GRASS),
        slab(at(52, -4, -8), at(60, 0, 8), GRASS),
        slab(at(60, -4, -8), at(76, 5, 8), STONE),
        slab(at(80, -4, -8), at(100, 5, 8), GRASS),
    ],
    movers: &[Mover {
        no: 1,
        home: Box::new(at(20, -2, -6), at(32, 0, 6)),
        step: Vec3::new(8, 0, 0),
        span: 128,
        color: WOOD,
    }],
    gems: &[
        Gem {
            no: 1,
            at: at(34, 5, 0),
        },
        Gem {
            no: 2,
            at: at(68, 9, 0),
        },
        Gem {
            no: 3,
            at: at(90, 9, 0),
        },
    ],
    crawlers: &[
        Crawler {
            no: 1,
            at: at(68, 5, 0),
            lo: q(62),
            hi: q(74),
            dir: 1,
            speed: 3,
        },
        Crawler {
            no: 2,
            at: at(88, 5, 0),
            lo: q(82),
            hi: q(94),
            dir: -1,
            speed: 2,
        },
    ],
    goal: at(96, 5, 0),
};

pub const fn level(n: LevelNo) -> Level {
    match n {
        LevelNo::One => MEADOW,
        LevelNo::Two => RIDGE,
    }
}
