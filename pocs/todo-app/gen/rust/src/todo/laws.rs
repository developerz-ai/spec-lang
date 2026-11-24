//! Module `todo.laws`, printed by spec-lang from `src/todo/laws.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.

use crate::rt::{self, Int, R};
use crate::machine::Machine;

/// Rule set `todo.laws.Change`, `src/todo/laws.slg:7`.
/// The calls of `todo.board.add`, `todo.board.done`, `todo.board.reopen`, `todo.board.rename`, `todo.board.remove`, `todo.board.clear`: a value of it is a `crate::machine::Event` of one of them.
pub type Change = crate::machine::Event;

/// a number is never reused
///
/// Law `todo.laws.numbers_below_next`, `src/todo/laws.slg:9`.
pub fn numbers_below_next(_m: &mut Machine, t: rt::runner::Trace) -> R<bool> {
    rt::list::all(
        &t.states,
        |w: crate::machine::World| -> R<bool> {
            rt::list::all(
                &w.tasks.keys().cloned().collect::<Vec<_>>(),
                |it: Int| -> R<bool> { Ok(it < w.next_no) },
            )
        },
    )
}

/// only add moves the counter, and only up
///
/// Law `todo.laws.next_only_grows`, `src/todo/laws.slg:13`.
pub fn next_only_grows(_m: &mut Machine, t: rt::runner::Trace) -> R<bool> {
    rt::list::all(
        &t.steps,
        |it: rt::runner::Step| -> R<bool> { Ok(it.after.next_no >= it.before.next_no) },
    )
}

/// a command that names a task touches no other task
///
/// Law `todo.laws.others_untouched`, `src/todo/laws.slg:17`.
pub fn others_untouched(_m: &mut Machine, t: rt::runner::Trace, k: Int) -> R<bool> {
    rt::list::all(
        &t.steps,
        |s: rt::runner::Step| -> R<bool> {
            Ok(
                !(match &s.event {
                    crate::machine::Event::Done { no: x, .. } => *x,
                    crate::machine::Event::Reopen { no: x, .. } => *x,
                    crate::machine::Event::Rename { no: x, .. } => *x,
                    crate::machine::Event::Remove { no: x, .. } => *x,
                    #[allow(unreachable_patterns)]
                    _ => {
                        return Err(
                            rt::stop("E-TYPE-MISMATCH", "the event has no argument `no`"),
                        );
                    }
                } != k)
                    || (rt::list::map_get(&s.after.tasks, &k)
                        == rt::list::map_get(&s.before.tasks, &k)),
            )
        },
    )
}

/// Law `todo.laws.clear_keeps_every_open_task`, `src/todo/laws.slg:21`.
pub fn clear_keeps_every_open_task(_m: &mut Machine, t: rt::runner::Trace) -> R<bool> {
    rt::list::all(
        &t.steps,
        |s: rt::runner::Step| -> R<bool> {
            Ok(
                !(matches!(&s.event, crate::machine::Event::Clear { .. }))
                    || (rt::list::all(
                        &s.after.tasks.values().cloned().collect::<Vec<_>>(),
                        |it: crate::todo::model::Task| -> R<bool> {
                            Ok(it.status == crate::todo::model::Status::Open)
                        },
                    )?
                        && (rt::int::count(s.after.tasks.len())
                            == rt::int::count(
                                rt::list::filter(
                                        &s.before.tasks.values().cloned().collect::<Vec<_>>(),
                                        |it: crate::todo::model::Task| -> R<bool> {
                                            Ok(it.status == crate::todo::model::Status::Open)
                                        },
                                    )?
                                    .len(),
                            ))),
            )
        },
    )
}

/// Law `todo.laws.reads_change_nothing`, `src/todo/laws.slg:26`.
pub fn reads_change_nothing(_m: &mut Machine, t: rt::runner::Trace) -> R<bool> {
    rt::list::all(
        &t.steps,
        |it: rt::runner::Step| -> R<bool> { Ok(it.after == it.before) },
    )
}

/// a refused command leaves the board as it was
///
/// Law `todo.laws.refusal_changes_nothing`, `src/todo/laws.slg:29`.
pub fn refusal_changes_nothing(_m: &mut Machine, t: rt::runner::Trace) -> R<bool> {
    rt::list::all(
        &t.steps,
        |s: rt::runner::Step| -> R<bool> {
            Ok(
                !(matches!(&s.reply, crate::machine::Reply::Err(_)))
                    || (s.after == s.before),
            )
        },
    )
}

/// a task is stored under its own number
///
/// Law `todo.laws.stored_under_its_number`, `src/todo/laws.slg:33`.
pub fn stored_under_its_number(_m: &mut Machine, t: rt::runner::Trace) -> R<bool> {
    rt::list::all(
        &t.states,
        |w: crate::machine::World| -> R<bool> {
            rt::list::all(
                &w.tasks.keys().cloned().collect::<Vec<_>>(),
                |k: Int| -> R<bool> { Ok(rt::list::map_at(&w.tasks, &k)?.no == k) },
            )
        },
    )
}
