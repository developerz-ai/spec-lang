// Projected by hand from module hop.tests.touches (src/hop/tests/touches.slg). Stand-in for spec-lang build output; change the spec, not this file.

mod support;

use std::collections::BTreeSet;

use skyhop::levels::{LevelNo, MEADOW, RIDGE};
use skyhop::model::Mode;
use skyhop::world::{Lines, Over, Silent, World};
use support::{REST, RIGHT, step};

/// `out.text`: every line the test's steps sent, in order.
#[test]
fn a_gem_is_taken_once() {
    let mut w = support::at_the_third_gem();
    let mut out = Lines::default();
    w.step(REST, &mut out).unwrap();
    assert_eq!(w.taken, BTreeSet::from([3]));
    w.step(REST, &mut out).unwrap();
    assert_eq!(w.taken, BTreeSet::from([3]));
    assert_eq!(out.text, "1 gem 3\n");
}

/// A life lost keeps the gems taken.
#[test]
fn a_fall_keeps_the_gems() {
    let mut w = support::about_to_fall_out_holding_a_gem();
    step(&mut w, REST).unwrap();
    assert_eq!(w.lives, 2);
    assert_eq!(w.taken, BTreeSet::from([1]));
}

/// Only a falling body stomps; rising into a crawler hurts.
#[test]
fn rising_into_a_crawler_hurts() {
    let mut w = support::rising_beside_the_crawler();
    step(&mut w, REST).unwrap();
    assert_eq!(w.lives, 2);
    assert!(w.crawlers.contains_key(&1));
}

/// Falling onto the upper half of a crawler removes it and bounces.
#[test]
fn a_stomp_removes_the_crawler() {
    let mut w = support::falling_on_the_crawler();
    step(&mut w, REST).unwrap();
    assert!(!w.crawlers.contains_key(&1));
    assert_eq!(w.player.vel.y, 24);
    assert_eq!(w.lives, 3);
}

/// Walking into a crawler costs a life and restarts the hero.
#[test]
fn a_crawler_hurts_from_the_side() {
    let mut w = support::running_at_the_crawler();
    step(&mut w, RIGHT).unwrap();
    assert_eq!(w.lives, 2);
    assert_eq!(w.player.at, MEADOW.start);
    assert!(w.crawlers.contains_key(&1));
}

#[test]
fn falling_out_costs_a_life() {
    let mut w = support::about_to_fall_out();
    step(&mut w, REST).unwrap();
    assert_eq!(w.lives, 2);
    assert_eq!(w.mode, Mode::Playing);
}

#[test]
fn the_last_life_ends_the_game() {
    let mut w = support::about_to_fall_out_for_the_last_time();
    step(&mut w, REST).unwrap();
    assert_eq!(w.lives, 0);
    assert_eq!(w.mode, Mode::Lost);
    assert_eq!(step(&mut w, REST), Err(Over));
}

#[test]
fn reaching_the_flag_wins() {
    let mut w = support::beside_the_flag();
    step(&mut w, REST).unwrap();
    assert_eq!(w.mode, Mode::Won);
    assert_eq!(step(&mut w, REST), Err(Over));
}

#[test]
fn starting_again_resets_everything() {
    let mut w = World::started(LevelNo::One, &mut Silent);
    for _ in 1..=5 {
        step(&mut w, RIGHT).unwrap();
    }
    w.start(LevelNo::Two, &mut Silent);
    assert_eq!(w.taken, BTreeSet::new());
    assert_eq!(w.ticks, 0);
    assert_eq!(w.crawlers.len(), 2);
    assert_eq!(w.player.at, RIDGE.start);
}
