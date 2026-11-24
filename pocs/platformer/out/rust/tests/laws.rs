// Projected by hand from module hop.laws: never_inside_a_solid, speed_is_bounded, taken_only_grows, lives_never_rise, lost_means_no_lives, over_is_final, one_tick_per_step, taken_are_real_gems (src/hop/laws.slg). Stand-in for spec-lang build output; change the spec, not this file.
//! Each law over seeded traces of `Play = start | step`: 240 traces of at least 200 events, from `start` of a random level,
//! then random pads held for random runs, and a fresh `start` now and then. Every law is checked after every event.

mod support;

use skyhop::geom;
use skyhop::levels::LevelNo;
use skyhop::model::{Mode, Pad};
use skyhop::physics;
use skyhop::world::{Lines, World};
use support::{MEADOW_WIN, Rng, pad_of};

const TRACES: u64 = 240;
const EVENTS: usize = 200;

/// Laws over `t.states`: they hold in every world a trace reaches.
fn state_laws(w: &World) -> Result<(), &'static str> {
    let body = physics::body(w.player.at);
    if w.stage.solids.iter().any(|s| geom::overlaps(body, s.r#box)) {
        return Err("never_inside_a_solid");
    }
    let v = w.player.vel;
    if !(v.x.abs() <= physics::RUN_SPEED
        && v.z.abs() <= physics::RUN_SPEED
        && -physics::MAX_FALL <= v.y
        && v.y <= physics::JUMP_SPEED)
    {
        return Err("speed_is_bounded");
    }
    if (w.lives == 0) != (w.mode == Mode::Lost) {
        return Err("lost_means_no_lives");
    }
    if !w
        .taken
        .iter()
        .all(|n| w.stage.gems.iter().any(|g| g.no == *n))
    {
        return Err("taken_are_real_gems");
    }
    Ok(())
}

/// Laws over `t.steps` of `Trace[world.step]`.
fn step_laws(before: &World, after: &World, accepted: bool) -> Result<(), &'static str> {
    if !before.taken.iter().all(|g| after.taken.contains(g)) {
        return Err("taken_only_grows");
    }
    if after.lives > before.lives {
        return Err("lives_never_rise");
    }
    if before.mode != Mode::Playing && (accepted || after != before) {
        return Err("over_is_final");
    }
    if accepted && after.ticks != before.ticks + 1 {
        return Err("one_tick_per_step");
    }
    Ok(())
}

fn level(rng: &mut Rng) -> LevelNo {
    if rng.bool() {
        LevelNo::One
    } else {
        LevelNo::Two
    }
}

/// What a trace saw, so the suite can show the laws were not checked only vacuously.
#[derive(Default)]
struct Reached {
    won: bool,
    lost: bool,
    refused: bool,
    taken: bool,
}

/// One trace. One trace in three opens with a known script on the meadow, so finished games are in the sample:
/// seed 0 mod 6 plays to the flag (316 steps), seed 1 mod 6 runs right off the first ledge until the last life (378 steps).
fn trace(seed: u64, reached: &mut Reached) -> Result<(), String> {
    let mut rng = Rng::new(0x5eed_0000 + seed);
    let mut out = Lines::default();
    let opening: Vec<Pad> = match seed % 6 {
        0 => MEADOW_WIN
            .iter()
            .flat_map(|(n, keys)| std::iter::repeat_n(pad_of(keys), *n as usize))
            .collect(),
        1 => vec![pad_of("R"); 380],
        _ => Vec::new(),
    };
    let first = if opening.is_empty() {
        level(&mut rng)
    } else {
        LevelNo::One
    };
    let mut w = World::started(first, &mut out);
    state_laws(&w).map_err(|law| format!("{law} after start"))?;
    let mut pad = Pad::default();
    let mut hold = 0;
    for event in 0..opening.len() + EVENTS {
        if let Some(scripted) = opening.get(event) {
            pad = *scripted;
        } else {
            if rng.int(0, 99) == 0 {
                w.start(level(&mut rng), &mut out);
                state_laws(&w).map_err(|law| format!("{law} after start at event {event}"))?;
                continue;
            }
            if hold == 0 {
                pad = rng.pad();
                hold = rng.int(1, 40);
            }
            hold -= 1;
        }
        let before = w.clone();
        let accepted = w.step(pad, &mut out).is_ok();
        let fail = |law| format!("{law} at event {event}, tick {}, pad {pad:?}", before.ticks);
        step_laws(&before, &w, accepted).map_err(fail)?;
        state_laws(&w).map_err(fail)?;
        reached.won |= w.mode == Mode::Won;
        reached.lost |= w.mode == Mode::Lost;
        reached.refused |= !accepted;
        reached.taken |= !w.taken.is_empty();
    }
    Ok(())
}

#[test]
fn every_law_after_every_event() {
    let mut reached = Reached::default();
    for seed in 0..TRACES {
        if let Err(why) = trace(seed, &mut reached) {
            panic!("seed {seed}: {why}");
        }
    }
    assert!(reached.won, "no trace won");
    assert!(reached.lost, "no trace lost");
    assert!(reached.refused, "no step was refused");
    assert!(reached.taken, "no gem was taken");
}
