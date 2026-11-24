//! Module `hop.laws`, printed by spec-lang from `src/hop/laws.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.

use crate::rt::{self, Int, R};
use crate::machine::Machine;

/// Rule set `hop.laws.Play`, `src/hop/laws.slg:8`.
/// The calls of `hop.world.start`, `hop.world.step`: a value of it is a `crate::machine::Event` of one of them.
pub type Play = crate::machine::Event;

/// the hero is never inside a solid
///
/// Law `hop.laws.never_inside_a_solid`, `src/hop/laws.slg:10`.
pub fn never_inside_a_solid(_m: &mut Machine, t: rt::runner::Trace) -> R<bool> {
    rt::list::all(
        &t.states,
        |w: crate::machine::World| -> R<bool> {
            rt::list::all(
                &w.stage.solids,
                |s: crate::hop::model::Solid| -> R<bool> {
                    Ok(
                        !crate::hop::geom::overlaps(
                            crate::hop::physics::body(w.player.at.clone())?,
                            s.r#box.clone(),
                        )?,
                    )
                },
            )
        },
    )
}

/// speeds stay in bounds, so a body never passes through a solid in one tick
///
/// Law `hop.laws.speed_is_bounded`, `src/hop/laws.slg:14`.
pub fn speed_is_bounded(_m: &mut Machine, t: rt::runner::Trace) -> R<bool> {
    rt::list::all(
        &t.states,
        |w: crate::machine::World| -> R<bool> {
            Ok(
                ((rt::int::abs(w.player.vel.x)? <= crate::hop::physics::RUN_SPEED)
                    && (rt::int::abs(w.player.vel.z)? <= crate::hop::physics::RUN_SPEED))
                    && {
                        let tmp1 = rt::int::sub(0, crate::hop::physics::MAX_FALL)?;
                        {
                            let tmp2 = w.player.vel.y;
                            tmp1 <= tmp2
                                && {
                                    let tmp3 = crate::hop::physics::JUMP_SPEED;
                                    tmp2 <= tmp3
                                }
                        }
                    },
            )
        },
    )
}

/// a gem once taken stays taken until the level starts again
///
/// Law `hop.laws.taken_only_grows`, `src/hop/laws.slg:19`.
pub fn taken_only_grows(_m: &mut Machine, t: rt::runner::Trace) -> R<bool> {
    rt::list::all(
        &t.steps,
        |s: rt::runner::Step| -> R<bool> {
            rt::list::all(
                &s.before.taken.iter().cloned().collect::<Vec<_>>(),
                |g: Int| -> R<bool> { Ok(s.after.taken.contains(&g)) },
            )
        },
    )
}

/// lives only go down during play
///
/// Law `hop.laws.lives_never_rise`, `src/hop/laws.slg:23`.
pub fn lives_never_rise(_m: &mut Machine, t: rt::runner::Trace) -> R<bool> {
    rt::list::all(
        &t.steps,
        |it: rt::runner::Step| -> R<bool> { Ok(it.after.lives <= it.before.lives) },
    )
}

/// no lives left is exactly a lost game
///
/// Law `hop.laws.lost_means_no_lives`, `src/hop/laws.slg:27`.
pub fn lost_means_no_lives(_m: &mut Machine, t: rt::runner::Trace) -> R<bool> {
    rt::list::all(
        &t.states,
        |w: crate::machine::World| -> R<bool> {
            Ok((w.lives == 0) == (w.mode == crate::hop::model::Mode::Lost))
        },
    )
}

/// a finished game is final: a step is refused and nothing moves
///
/// Law `hop.laws.over_is_final`, `src/hop/laws.slg:31`.
pub fn over_is_final(_m: &mut Machine, t: rt::runner::Trace) -> R<bool> {
    rt::list::all(
        &t.steps,
        |s: rt::runner::Step| -> R<bool> {
            Ok(
                !(s.before.mode != crate::hop::model::Mode::Playing)
                    || ((matches!(&s.reply, crate::machine::Reply::Err(_)))
                        && (s.after == s.before)),
            )
        },
    )
}

/// time moves one tick per accepted step
///
/// Law `hop.laws.one_tick_per_step`, `src/hop/laws.slg:35`.
pub fn one_tick_per_step(_m: &mut Machine, t: rt::runner::Trace) -> R<bool> {
    rt::list::all(
        &t.steps,
        |s: rt::runner::Step| -> R<bool> {
            Ok(
                !(matches!(&s.reply, crate::machine::Reply::Ok(_)))
                    || (s.after.ticks == rt::int::add(s.before.ticks, 1)?),
            )
        },
    )
}

/// a taken gem is one the level has
///
/// Law `hop.laws.taken_are_real_gems`, `src/hop/laws.slg:39`.
pub fn taken_are_real_gems(_m: &mut Machine, t: rt::runner::Trace) -> R<bool> {
    rt::list::all(
        &t.states,
        |w: crate::machine::World| -> R<bool> {
            rt::list::all(
                &w.taken.iter().cloned().collect::<Vec<_>>(),
                |n: Int| -> R<bool> {
                    rt::list::any(
                        &w.stage.gems,
                        |it: crate::hop::model::Gem| -> R<bool> { Ok(it.no == n) },
                    )
                },
            )
        },
    )
}
