// Projected by hand from the named start worlds past_the_first_ledge, running_at_the_step, under_a_low_ceiling, riding_the_mover, above_the_mover, at_the_third_gem, falling_on_the_crawler, running_at_the_crawler, about_to_fall_out, about_to_fall_out_for_the_last_time, beside_the_flag, about_to_fall_out_holding_a_gem, rising_beside_the_crawler, under_the_mover, falling_fast_onto_the_floor (src/hop/world.slg), pad_of, Hold, meadow_win, ridge_win (src/hop/tests/runs.slg), and the generators behind law and property parameters. Stand-in for spec-lang build output; change the spec, not this file.
#![allow(dead_code)]

use std::collections::{BTreeMap, BTreeSet};

use skyhop::geom::{Box, Vec3};
use skyhop::levels::{MEADOW, RIDGE};
use skyhop::model::{Crawler, Level, Pad, Player, Rgb, Solid};
use skyhop::world::{Lines, Over, World};

/// SplitMix64: the seeded generator behind every property and law test.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    pub const fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    /// Uniform in `lo..=hi`.
    pub fn int(&mut self, lo: i64, hi: i64) -> i64 {
        let span = (hi - lo + 1) as u64;
        lo + (self.next() % span) as i64
    }

    pub fn bool(&mut self) -> bool {
        self.next() & 1 == 1
    }

    pub fn pad(&mut self) -> Pad {
        let bits = self.next();
        Pad {
            left: bits & 1 != 0,
            right: bits & 2 != 0,
            forward: bits & 4 != 0,
            back: bits & 8 != 0,
            jump: bits & 16 != 0,
        }
    }
}

/// `step` with its lines dropped, as a scenario calls it.
pub fn step(world: &mut World, pad: Pad) -> Result<(), Over> {
    world.step(pad, &mut Lines::default())
}

pub const REST: Pad = Pad {
    left: false,
    right: false,
    forward: false,
    back: false,
    jump: false,
};
pub const RIGHT: Pad = Pad {
    right: true,
    ..REST
};
pub const JUMP: Pad = Pad { jump: true, ..REST };

/// One letter per Pad field, as the game vocabulary reads a script: L R F B J, or - for none.
pub fn pad_of(keys: &str) -> Pad {
    Pad {
        left: keys.contains('L'),
        right: keys.contains('R'),
        forward: keys.contains('F'),
        back: keys.contains('B'),
        jump: keys.contains('J'),
    }
}

/// `type Hold(ticks: 1..=10000, keys: Text)`.
pub type Hold = (u32, &'static str);

pub const MEADOW_WIN: &[Hold] = &[
    (75, "R"),
    (20, "RJ"),
    (41, "R"),
    (20, "RJ"),
    (35, "R"),
    (20, "RJ"),
    (23, "R"),
    (20, "RJ"),
    (62, "R"),
];

pub const RIDGE_WIN: &[Hold] = &[
    (24, "R"),
    (8, "-"),
    (8, "R"),
    (16, "RJ"),
    (16, "R"),
    (8, "-"),
    (8, "L"),
    (8, "R"),
    (8, "L"),
    (8, "R"),
    (8, "L"),
    (8, "R"),
    (8, "L"),
    (8, "R"),
    (8, "L"),
    (8, "R"),
    (16, "RJ"),
    (24, "R"),
    (16, "RJ"),
    (8, "-"),
    (8, "R"),
    (16, "-"),
    (16, "J"),
    (8, "-"),
    (16, "R"),
    (8, "J"),
    (8, "L"),
    (24, "R"),
    (16, "RJ"),
    (32, "R"),
    (8, "RJ"),
    (8, "J"),
    (8, "R"),
    (8, "-"),
    (16, "RJ"),
    (32, "R"),
    (16, "RJ"),
    (16, "R"),
    (8, "-"),
    (32, "R"),
    (16, "RJ"),
    (8, "R"),
];

/// A `@given` world: the declared initial values, then the named states replaced.
fn given(stage: Level, player: Player) -> World {
    World {
        stage,
        player,
        ..World::default()
    }
}

fn grounded(at: Vec3) -> Player {
    Player {
        grounded: true,
        ..Player::new(at)
    }
}

/// A body just past the first ledge of the meadow, still within its coyote ticks.
pub fn past_the_first_ledge() -> World {
    given(
        MEADOW,
        Player {
            coyote: 6,
            ..grounded(Vec3::new(1090, 0, 0))
        },
    )
}

/// Running at the stone step of the meadow.
pub fn running_at_the_step() -> World {
    given(
        MEADOW,
        Player {
            vel: Vec3::new(16, 0, 0),
            ..grounded(Vec3::new(2400, 0, 0))
        },
    )
}

const BLACK: Rgb = Rgb::new(0, 0, 0);
const LOW_ROOM: Level = Level {
    name: "box",
    start: Vec3::new(0, 0, 0),
    solids: &[
        Solid {
            r#box: Box::new(Vec3::new(-512, -256, -512), Vec3::new(512, 0, 512)),
            color: BLACK,
        },
        Solid {
            r#box: Box::new(Vec3::new(-512, 400, -512), Vec3::new(512, 600, 512)),
            color: BLACK,
        },
    ],
    movers: &[],
    gems: &[],
    crawlers: &[],
    goal: Vec3::new(100_000, 0, 0),
};

/// A one-metre room whose ceiling is lower than a jump.
pub fn under_a_low_ceiling() -> World {
    given(LOW_ROOM, grounded(Vec3::new(0, 0, 0)))
}

pub fn riding_the_mover() -> World {
    given(
        RIDGE,
        Player {
            riding: Some(1),
            ..grounded(Vec3::new(1600, 0, 0))
        },
    )
}

pub fn above_the_mover() -> World {
    given(RIDGE, Player::new(Vec3::new(1600, 200, 0)))
}

pub fn at_the_third_gem() -> World {
    given(MEADOW, grounded(Vec3::new(4608, 0, 0)))
}

const ONE_CRAWLER: Crawler = Crawler {
    no: 1,
    at: Vec3::new(3072, 256, 0),
    lo: 2752,
    hi: 3392,
    dir: 1,
    speed: 2,
};

pub fn falling_on_the_crawler() -> World {
    World {
        crawlers: BTreeMap::from([(1, ONE_CRAWLER)]),
        ..given(
            MEADOW,
            Player {
                vel: Vec3::new(0, -20, 0),
                ..Player::new(Vec3::new(3072, 400, 0))
            },
        )
    }
}

pub fn running_at_the_crawler() -> World {
    World {
        crawlers: BTreeMap::from([(1, ONE_CRAWLER)]),
        ..given(
            MEADOW,
            Player {
                vel: Vec3::new(16, 0, 0),
                ..grounded(Vec3::new(2900, 256, 0))
            },
        )
    }
}

pub fn about_to_fall_out() -> World {
    given(
        MEADOW,
        Player {
            vel: Vec3::new(0, -48, 0),
            ..Player::new(Vec3::new(1280, -2040, 0))
        },
    )
}

pub fn about_to_fall_out_for_the_last_time() -> World {
    World {
        lives: 1,
        ..about_to_fall_out()
    }
}

pub fn beside_the_flag() -> World {
    given(MEADOW, grounded(Vec3::new(5000, 0, 0)))
}

/// A gem already taken, then a fall.
pub fn about_to_fall_out_holding_a_gem() -> World {
    World {
        taken: BTreeSet::from([1]),
        ..about_to_fall_out()
    }
}

/// Rising beside a crawler, feet above its middle.
pub fn rising_beside_the_crawler() -> World {
    World {
        crawlers: BTreeMap::from([(1, ONE_CRAWLER)]),
        ..given(
            MEADOW,
            Player {
                vel: Vec3::new(0, 20, 0),
                ..Player::new(Vec3::new(2920, 330, 0))
            },
        )
    }
}

/// Under the mover, inside its thickness, falling.
pub fn under_the_mover() -> World {
    given(RIDGE, Player::new(Vec3::new(1600, -200, 0)))
}

/// Falling as fast as anything falls, onto the first floor.
pub fn falling_fast_onto_the_floor() -> World {
    given(
        MEADOW,
        Player {
            vel: Vec3::new(0, -48, 0),
            ..Player::new(Vec3::new(0, 400, 0))
        },
    )
}
