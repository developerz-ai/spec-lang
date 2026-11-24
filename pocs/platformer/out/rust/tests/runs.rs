// Projected by hand from module hop.tests.runs: pad_of and its @cases, the_meadow_can_be_won, the_ridge_can_be_won (src/hop/tests/runs.slg). Stand-in for spec-lang build output; change the spec, not this file.

mod support;

use skyhop::levels::LevelNo;
use skyhop::model::{Mode, Pad};
use skyhop::world::{Silent, World};
use support::{MEADOW_WIN, RIDGE_WIN, pad_of, step};

#[test]
fn pad_of_cases() {
    let cases = [
        ("-", Pad::default()),
        (
            "RJ",
            Pad {
                right: true,
                jump: true,
                ..Pad::default()
            },
        ),
        (
            "LFB",
            Pad {
                left: true,
                forward: true,
                back: true,
                ..Pad::default()
            },
        ),
    ];
    for (keys, want) in cases {
        assert_eq!(pad_of(keys), want, "pad_of({keys:?})");
    }
}

/// The meadow is winnable, with every gem, without losing a life.
#[test]
fn the_meadow_can_be_won() {
    let mut w = World::started(LevelNo::One, &mut Silent);
    for (ticks, keys) in MEADOW_WIN {
        for _ in 1..=*ticks {
            step(&mut w, pad_of(keys)).unwrap();
        }
    }
    assert_eq!(w.mode, Mode::Won);
    assert_eq!(w.ticks, 316);
    assert_eq!(w.taken.len(), 3);
    assert_eq!(w.lives, 3);
}

/// The ridge is winnable too.
#[test]
fn the_ridge_can_be_won() {
    let mut w = World::started(LevelNo::Two, &mut Silent);
    for (ticks, keys) in RIDGE_WIN {
        for _ in 1..=*ticks {
            if w.mode == Mode::Playing {
                step(&mut w, pad_of(keys)).unwrap();
            }
        }
    }
    assert_eq!(w.mode, Mode::Won);
    assert_eq!(w.ticks, 549);
    assert_eq!(w.taken.len(), 3);
    assert_eq!(w.lives, 3);
}
