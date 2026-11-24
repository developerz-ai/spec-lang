//! Module `todo.tests.scenarios`, printed by spec-lang from `src/todo/tests/scenarios.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.
use crate::rt::{self, Int, Opt, R};
use crate::machine::Machine;
use std::collections::BTreeSet;
/// Test `todo.tests.scenarios.numbers_start_at_one`, `src/todo/tests/scenarios.slg:8`.
pub fn numbers_start_at_one(m: &mut Machine) -> R<()> {
    let tmp1 = rt::settle(
        rt::catch(|| crate::todo::board::add(m, String::from("milk"), None, None)),
    )?;
    rt::expect_value(tmp1, 1)?;
    let tmp2 = rt::settle(
        rt::catch(|| crate::todo::board::add(
            m,
            String::from("bread"),
            None,
            Some(BTreeSet::from([String::from("shop"), String::from("home")])),
        )),
    )?;
    rt::expect_value(tmp2, 2)?;
    rt::assert(crate::machine::out_text(m)? == "added 1\nadded 2\n")?;
    Ok(())
}
/// Test `todo.tests.scenarios.numbers_are_never_reused`, `src/todo/tests/scenarios.slg:14`.
pub fn numbers_are_never_reused(m: &mut Machine) -> R<()> {
    let tmp1 = rt::settle(
        rt::catch(|| crate::todo::board::add(m, String::from("a"), None, None)),
    )?;
    rt::expect_value(tmp1, 1)?;
    let tmp2 = rt::settle(
        rt::catch(|| crate::todo::board::add(m, String::from("b"), None, None)),
    )?;
    rt::expect_value(tmp2, 2)?;
    crate::todo::board::remove(m, 2)?;
    let tmp3 = rt::settle(
        rt::catch(|| crate::todo::board::add(m, String::from("c"), None, None)),
    )?;
    rt::expect_value(tmp3, 3)?;
    Ok(())
}
/// Test `todo.tests.scenarios.empty_board_says_so`, `src/todo/tests/scenarios.slg:21`.
pub fn empty_board_says_so(m: &mut Machine) -> R<()> {
    crate::todo::board::list(m, None, None)?;
    crate::todo::board::stats(m)?;
    rt::assert(crate::machine::out_text(m)? == "nothing to do\n0 open, 0 done\n")?;
    Ok(())
}
/// Test `todo.tests.scenarios.list_ranks_urgent_first`, `src/todo/tests/scenarios.slg:27`.
pub fn list_ranks_urgent_first(m: &mut Machine) -> R<()> {
    crate::todo::board::list(m, None, None)?;
    rt::assert(
        crate::machine::out_text(m)?
            == "3 [ ] urgent !high #work\n2 [ ] plain\n1 [ ] low one !low\n",
    )?;
    Ok(())
}
/// Test `todo.tests.scenarios.list_all_and_by_tag`, `src/todo/tests/scenarios.slg:33`.
pub fn list_all_and_by_tag(m: &mut Machine) -> R<()> {
    crate::todo::board::list(m, Some(true), Some(Opt::Some(String::from("work"))))?;
    rt::assert(
        crate::machine::out_text(m)?
            == "3 [ ] urgent !high #work\n4 [x] finished #home #work\n",
    )?;
    Ok(())
}
/// Test `todo.tests.scenarios.done_and_reopen_move_once`, `src/todo/tests/scenarios.slg:39`.
pub fn done_and_reopen_move_once(m: &mut Machine) -> R<()> {
    crate::todo::board::done(m, 2)?;
    let tmp1 = rt::settle(rt::catch(|| crate::todo::board::done(m, 2)))?;
    rt::expect_fail(tmp1, "AlreadyDone", vec![])?;
    crate::todo::board::reopen(m, 2)?;
    let tmp2 = rt::settle(rt::catch(|| crate::todo::board::reopen(m, 2)))?;
    rt::expect_fail(tmp2, "NotDone", vec![])?;
    rt::assert(crate::machine::out_text(m)? == "2 [x] plain\n2 [ ] plain\n")?;
    Ok(())
}
/// Test `todo.tests.scenarios.rename_keeps_the_rest`, `src/todo/tests/scenarios.slg:48`.
pub fn rename_keeps_the_rest(m: &mut Machine) -> R<()> {
    crate::todo::board::rename(m, 3, String::from("urgent, really"))?;
    rt::assert(
        rt::list::map_at(&m.world.tasks, &3)?
            == crate::todo::model::Task::new(
                3,
                String::from("urgent, really"),
                None,
                Some(crate::todo::model::Priority::High),
                Some(BTreeSet::from([String::from("work")])),
            )?,
    )?;
    Ok(())
}
/// Test `todo.tests.scenarios.clear_removes_only_done`, `src/todo/tests/scenarios.slg:54`.
pub fn clear_removes_only_done(m: &mut Machine) -> R<()> {
    let tmp1 = rt::settle(rt::catch(|| crate::todo::board::clear(m)))?;
    rt::expect_value(tmp1, 1)?;
    crate::todo::board::stats(m)?;
    rt::assert(crate::machine::out_text(m)? == "cleared 1\n3 open, 0 done\n")?;
    rt::assert(!m.world.tasks.contains_key(&4))?;
    Ok(())
}
/// Test `todo.tests.scenarios.unknown_numbers_are_refused`, `src/todo/tests/scenarios.slg:62`.
pub fn unknown_numbers_are_refused(m: &mut Machine, no: Int) -> R<()> {
    if no <= 4 {
        {
            let _ = ();
            return Ok(());
        }
    }
    let tmp1 = rt::settle(rt::catch(|| crate::todo::board::done(m, no)))?;
    rt::expect_fail(tmp1, "NoTask", vec![])?;
    let tmp2 = rt::settle(rt::catch(|| crate::todo::board::reopen(m, no)))?;
    rt::expect_fail(tmp2, "NoTask", vec![])?;
    let tmp3 = rt::settle(
        rt::catch(|| crate::todo::board::rename(m, no, String::from("x"))),
    )?;
    rt::expect_fail(tmp3, "NoTask", vec![])?;
    let tmp4 = rt::settle(rt::catch(|| crate::todo::board::remove(m, no)))?;
    rt::expect_fail(tmp4, "NoTask", vec![])?;
    rt::assert(crate::machine::out_text(m)?.is_empty())?;
    Ok(())
}
