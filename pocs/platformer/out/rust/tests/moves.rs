// Projected by hand from module hop.tests.moves (src/hop/tests/moves.slg). Stand-in for spec-lang build output; change the spec, not this file.

mod support;

use skyhop::geom::Vec3;
use skyhop::levels::LevelNo;
use skyhop::world::{Silent, World};
use support::{JUMP, REST, RIGHT, step};

fn started() -> World {
    World::started(LevelNo::One, &mut Silent)
}

#[test]
fn standing_still_stays_put() {
    let mut w = started();
    for _ in 1..=30 {
        step(&mut w, REST).unwrap();
    }
    assert_eq!(w.player.at, Vec3::new(0, 0, 0));
    assert!(w.player.grounded);
    assert_eq!(w.ticks, 30);
}

/// The first tick starts in the air, so it gains 1; then 2 a tick up to 16.
#[test]
fn running_reaches_full_speed() {
    let mut w = started();
    for _ in 1..=10 {
        step(&mut w, RIGHT).unwrap();
    }
    assert_eq!(w.player.vel.x, 16);
    assert_eq!(w.player.at.x, 96);
    assert_eq!(w.player.yaw, 0);
}

/// A held jump rises 38 + 36 + ... + 2 = 380 sub-metres.
#[test]
fn a_held_jump_peaks_at_380() {
    let mut w = started();
    step(&mut w, REST).unwrap();
    let mut top = 0;
    for _ in 1..=50 {
        step(&mut w, JUMP).unwrap();
        top = top.max(w.player.at.y);
    }
    assert_eq!(top, 380);
    assert!(w.player.grounded);
}

/// Let go after one tick: 38, then cut to 16 and fall away: 14 + 12 + ... + 2 more.
#[test]
fn a_short_hop_peaks_at_94() {
    let mut w = started();
    step(&mut w, REST).unwrap();
    step(&mut w, JUMP).unwrap();
    let mut top = 0;
    for _ in 1..=30 {
        step(&mut w, REST).unwrap();
        top = top.max(w.player.at.y);
    }
    assert_eq!(top, 94);
}

/// Holding jump through a landing does not jump again.
#[test]
fn a_jump_needs_a_fresh_press() {
    let mut w = started();
    step(&mut w, REST).unwrap();
    for _ in 1..=60 {
        step(&mut w, JUMP).unwrap();
    }
    assert!(w.player.grounded);
    assert_eq!(w.player.at.y, 0);
}

#[test]
fn a_jump_just_after_the_edge_still_counts() {
    let mut w = support::past_the_first_ledge();
    step(&mut w, REST).unwrap();
    assert!(!w.player.grounded);
    step(&mut w, JUMP).unwrap();
    assert_eq!(w.player.vel.y, 38);
}

#[test]
fn a_jump_long_after_the_edge_does_not() {
    let mut w = support::past_the_first_ledge();
    for _ in 1..=7 {
        step(&mut w, REST).unwrap();
    }
    step(&mut w, JUMP).unwrap();
    assert!(w.player.vel.y < 0);
}

#[test]
fn a_wall_stops_the_body_flush() {
    let mut w = support::running_at_the_step();
    for _ in 1..=10 {
        step(&mut w, RIGHT).unwrap();
    }
    assert_eq!(w.player.at.x, 2496);
    assert_eq!(w.player.vel.x, 0);
}

#[test]
fn a_ceiling_stops_a_jump() {
    let mut w = support::under_a_low_ceiling();
    let mut top = 0;
    for _ in 1..=40 {
        step(&mut w, JUMP).unwrap();
        top = top.max(w.player.at.y);
    }
    assert_eq!(top, 208);
}

#[test]
fn a_rider_moves_with_the_mover() {
    let mut w = support::riding_the_mover();
    for _ in 1..=10 {
        step(&mut w, REST).unwrap();
    }
    assert_eq!(w.player.at, Vec3::new(1680, 0, 0));
    assert_eq!(w.player.riding, Some(1));
}

#[test]
fn a_mover_catches_a_falling_body() {
    let mut w = support::above_the_mover();
    for _ in 1..=40 {
        step(&mut w, REST).unwrap();
    }
    assert_eq!(w.player.riding, Some(1));
    assert_eq!(w.player.at.y, 0);
}

/// A mover is one-way: a body below it keeps falling.
#[test]
fn a_mover_never_catches_from_below() {
    let mut w = support::under_the_mover();
    step(&mut w, REST).unwrap();
    assert!(w.player.at.y < -200);
    assert_eq!(w.player.riding, None);
}

/// The fastest fall still stops on a floor.
#[test]
fn a_fast_fall_lands() {
    let mut w = support::falling_fast_onto_the_floor();
    for _ in 1..=12 {
        step(&mut w, REST).unwrap();
    }
    assert_eq!(w.player.at.y, 0);
    assert!(w.player.grounded);
}
