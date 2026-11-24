// Projected by hand from the start-world tests past_the_first_ledge, running_at_the_step, under_a_low_ceiling, riding_the_mover, above_the_mover, at_the_third_gem, falling_on_the_crawler, running_at_the_crawler, about_to_fall_out, about_to_fall_out_for_the_last_time, beside_the_flag, about_to_fall_out_holding_a_gem, rising_beside_the_crawler, under_the_mover, falling_fast_onto_the_floor (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
//! Each start world is a test with an empty body: it passes when its `@given` builds a world its types allow.

mod support;

use skyhop::world::World;

/// The state types' ranges: `lives: 0..=3`, `coyote: 0..=6`, `yaw: 0..=359`, `Sub` coordinates.
fn admitted(w: &World) -> bool {
    let sub = |v: i32| (-1_048_576..=1_048_576).contains(&v);
    let p = &w.player;
    (0..=3).contains(&w.lives)
        && (0..=6).contains(&p.coyote)
        && (0..=359).contains(&p.yaw)
        && [p.at.x, p.at.y, p.at.z, p.vel.x, p.vel.y, p.vel.z]
            .into_iter()
            .all(sub)
}

macro_rules! start_world {
    ($($name:ident),* $(,)?) => {$(
        #[test]
        fn $name() {
            assert!(admitted(&support::$name()));
        }
    )*};
}

start_world!(
    past_the_first_ledge,
    running_at_the_step,
    under_a_low_ceiling,
    riding_the_mover,
    above_the_mover,
    at_the_third_gem,
    falling_on_the_crawler,
    running_at_the_crawler,
    about_to_fall_out,
    about_to_fall_out_for_the_last_time,
    beside_the_flag,
    about_to_fall_out_holding_a_gem,
    rising_beside_the_crawler,
    under_the_mover,
    falling_fast_onto_the_floor,
);
