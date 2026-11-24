//! Module `todo.board`, printed by spec-lang from `src/todo/board.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.
use crate::rt::{self, Int, Opt, R, Show};
use crate::machine::Machine;
use std::collections::BTreeSet;
/// Fn `todo.board.has_tag`, `src/todo/board.slg:16`.
pub fn has_tag(t: crate::todo::model::Task, tag: Opt<String>) -> R<bool> {
    rt::claim(crate::conform::task(&t)?, "argument `t` of todo.board.has_tag")?;
    rt::claim(
        crate::conform::option_tag(&tag)?,
        "argument `tag` of todo.board.has_tag",
    )?;
    Ok(
        if let Opt::None = &tag {
            true
        } else if let Opt::Some(_) = &tag && let Opt::Some(tmp1) = &tag {
            let x = (*tmp1).clone();
            t.tags.contains(&x)
        } else {
            return Err(rt::stop("E-RUNTIME", "no arm matches"))
        },
    )
}
/// a number is never reused, so an old note that says 'task 3' never names a new task
///
/// Rule `todo.board.add`, `src/todo/board.slg:21`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn add(
    m: &mut Machine,
    title: String,
    priority: Option<crate::todo::model::Priority>,
    tags: Option<BTreeSet<String>>,
) -> R<Int> {
    m.atomic(|m| {
        let priority = match priority {
            Some(v) => v,
            None => crate::todo::model::Priority::Normal,
        };
        let tags = tags.unwrap_or_default();
        rt::claim(crate::conform::title(&title)?, "argument `title` of todo.board.add")?;
        rt::claim(crate::conform::set_tag(&tags)?, "argument `tags` of todo.board.add")?;
        let result: Int = {
            let no: Int = m.world.next_no;
            let tmp1 = crate::todo::model::Task::new(
                no,
                title.clone(),
                None,
                Some(priority.clone()),
                Some(tags.clone()),
            )?;
            rt::claim(crate::conform::task(&tmp1)?, "a write to todo.board.tasks")?;
            m.world.tasks.insert(no, tmp1);
            let tmp2 = rt::int::add(no, 1)?;
            rt::claim(crate::conform::task_no(&tmp2)?, "a write to todo.board.next_no")?;
            m.world.next_no = tmp2;
            crate::machine::out_line(m, format!("added {}", no.show()))?;
            no
        };
        rt::claim(crate::conform::task_no(&result)?, "the result of todo.board.add")?;
        Ok(result)
    })
}
/// open work first; finished work only when asked for
///
/// Rule `todo.board.list`, `src/todo/board.slg:32`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn list(m: &mut Machine, all: Option<bool>, tag: Option<Opt<String>>) -> R<()> {
    m.atomic(|m| {
        let all = all.unwrap_or_default();
        let tag = tag.unwrap_or_default();
        rt::claim(
            crate::conform::option_tag(&tag)?,
            "argument `tag` of todo.board.list",
        )?;
        let shown: Vec<crate::todo::model::Task> = rt::list::sort_by(
            &rt::list::filter(
                &m.world.tasks.values().cloned().collect::<Vec<_>>(),
                |t: crate::todo::model::Task| -> R<bool> {
                    Ok(
                        (all || (t.status == crate::todo::model::Status::Open))
                            && has_tag(t.clone(), tag.clone())?,
                    )
                },
            )?,
            |t: crate::todo::model::Task| -> R<(Int, Int)> {
                crate::todo::render::rank(t.clone())
            },
            false,
        )?;
        if rt::int::count(shown.len()) == 0 {
            crate::machine::out_line(m, String::from("nothing to do"))?;
            return Ok(());
        }
        for t in shown.clone() {
            crate::machine::out_line(m, crate::todo::render::line(t.clone())?)?;
        }
        Ok(())
    })
}
/// finishing is one step, and only open work can finish
///
/// Rule `todo.board.done`, `src/todo/board.slg:44`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn done(m: &mut Machine, no: Int) -> R<()> {
    m.atomic(|m| {
        rt::claim(crate::conform::task_no(&no)?, "argument `no` of todo.board.done")?;
        if !m.world.tasks.contains_key(&no) {
            return Err(rt::fail("NoTask", Vec::new()));
        }
        rt::guard(
            rt::catch(|| Ok(
                rt::list::map_at(&m.world.tasks, &no)?.status
                    == crate::todo::model::Status::Open,
            )),
            "AlreadyDone",
        )?;
        rt::slot(&mut m.world.tasks, &no)?.status = crate::todo::model::Status::Done;
        let tmp1 = crate::todo::render::line(rt::list::map_at(&m.world.tasks, &no)?)?;
        crate::machine::out_line(m, tmp1)?;
        Ok(())
    })
}
/// a mistaken done is undone, not re-added under a new number
///
/// Rule `todo.board.reopen`, `src/todo/board.slg:53`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn reopen(m: &mut Machine, no: Int) -> R<()> {
    m.atomic(|m| {
        rt::claim(crate::conform::task_no(&no)?, "argument `no` of todo.board.reopen")?;
        if !m.world.tasks.contains_key(&no) {
            return Err(rt::fail("NoTask", Vec::new()));
        }
        rt::guard(
            rt::catch(|| Ok(
                rt::list::map_at(&m.world.tasks, &no)?.status
                    == crate::todo::model::Status::Done,
            )),
            "NotDone",
        )?;
        rt::slot(&mut m.world.tasks, &no)?.status = crate::todo::model::Status::Open;
        let tmp1 = crate::todo::render::line(rt::list::map_at(&m.world.tasks, &no)?)?;
        crate::machine::out_line(m, tmp1)?;
        Ok(())
    })
}
/// a title changes without losing its number, status or tags
///
/// Rule `todo.board.rename`, `src/todo/board.slg:62`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn rename(m: &mut Machine, no: Int, title: String) -> R<()> {
    m.atomic(|m| {
        rt::claim(crate::conform::task_no(&no)?, "argument `no` of todo.board.rename")?;
        rt::claim(
            crate::conform::title(&title)?,
            "argument `title` of todo.board.rename",
        )?;
        if !m.world.tasks.contains_key(&no) {
            return Err(rt::fail("NoTask", Vec::new()));
        }
        let tmp1 = title.clone();
        rt::claim(crate::conform::title(&tmp1)?, "a write to todo.board.tasks")?;
        rt::slot(&mut m.world.tasks, &no)?.title = tmp1;
        let tmp2 = crate::todo::render::line(rt::list::map_at(&m.world.tasks, &no)?)?;
        crate::machine::out_line(m, tmp2)?;
        Ok(())
    })
}
/// removing frees nothing: the number stays used
///
/// Rule `todo.board.remove`, `src/todo/board.slg:71`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn remove(m: &mut Machine, no: Int) -> R<()> {
    m.atomic(|m| {
        rt::claim(crate::conform::task_no(&no)?, "argument `no` of todo.board.remove")?;
        if !m.world.tasks.contains_key(&no) {
            return Err(rt::fail("NoTask", Vec::new()));
        }
        let tmp1 = m.world.tasks.clone();
        let tmp2 = rt::list::map_remove(&tmp1, &no);
        rt::claim(
            crate::conform::map_task_no_task(&tmp2)?,
            "a write to todo.board.tasks",
        )?;
        m.world.tasks = tmp2;
        crate::machine::out_line(m, format!("removed {}", no.show()))?;
        Ok(())
    })
}
/// finished work leaves the list in one step
///
/// Rule `todo.board.clear`, `src/todo/board.slg:80`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn clear(m: &mut Machine) -> R<Int> {
    m.atomic(|m| {
        let gone: Vec<crate::todo::model::Task> = rt::list::filter(
            &m.world.tasks.values().cloned().collect::<Vec<_>>(),
            |it: crate::todo::model::Task| -> R<bool> {
                Ok(it.status == crate::todo::model::Status::Done)
            },
        )?;
        for t in gone.clone() {
            let tmp1 = m.world.tasks.clone();
            let tmp2 = rt::list::map_remove(&tmp1, &t.no);
            rt::claim(
                crate::conform::map_task_no_task(&tmp2)?,
                "a write to todo.board.tasks",
            )?;
            m.world.tasks = tmp2;
        }
        crate::machine::out_line(
            m,
            format!("cleared {}", rt::int::count(gone.len()).show()),
        )?;
        Ok(rt::int::count(gone.len()))
    })
}
/// a count at a glance, without listing
///
/// Rule `todo.board.stats`, `src/todo/board.slg:90`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn stats(m: &mut Machine) -> R<()> {
    m.atomic(|m| {
        let open: Int = rt::int::count(
            rt::list::filter(
                    &m.world.tasks.values().cloned().collect::<Vec<_>>(),
                    |it: crate::todo::model::Task| -> R<bool> {
                        Ok(it.status == crate::todo::model::Status::Open)
                    },
                )?
                .len(),
        );
        let tmp1 = open.show();
        let tmp2 = format!(
            "{} open, {} done",
            tmp1,
            rt::int::sub(rt::int::count(m.world.tasks.len()), open)?.show(),
        );
        crate::machine::out_line(m, tmp2)?;
        Ok(())
    })
}
/// a named start world for tests in other modules: three open tasks, one done
///
/// Test `todo.board.three_open_one_done`, `src/todo/board.slg:98`.
pub fn three_open_one_done(m: &mut Machine) -> R<()> {
    Ok(())
}
