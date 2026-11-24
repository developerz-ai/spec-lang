//! Module `todo.render`, printed by spec-lang from `src/todo/render.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.
use crate::rt::{self, Int, R, Show};
use std::collections::BTreeSet;
/// Fn `todo.render.mark`, `src/todo/render.slg:8`.
pub fn mark(s: crate::todo::model::Status) -> R<String> {
    Ok(
        if let crate::todo::model::Status::Open = &s {
            String::from(" ")
        } else if let crate::todo::model::Status::Done = &s {
            String::from("x")
        } else {
            return Err(rt::stop("E-RUNTIME", "no arm matches"))
        },
    )
}
/// Fn `todo.render.badge`, `src/todo/render.slg:13`.
pub fn badge(p: crate::todo::model::Priority) -> R<String> {
    Ok(
        if let crate::todo::model::Priority::Low = &p {
            String::from(" !low")
        } else if let crate::todo::model::Priority::Normal = &p {
            String::from("")
        } else if let crate::todo::model::Priority::High = &p {
            String::from(" !high")
        } else {
            return Err(rt::stop("E-RUNTIME", "no arm matches"))
        },
    )
}
/// Fn `todo.render.tag_text`, `src/todo/render.slg:19`.
pub fn tag_text(tags: BTreeSet<String>) -> R<String> {
    rt::claim(
        crate::conform::set_tag(&tags)?,
        "argument `tags` of todo.render.tag_text",
    )?;
    Ok(
        rt::list::map(
                &tags.iter().cloned().collect::<Vec<_>>(),
                |t: String| -> R<String> { Ok(format!(" #{}", t)) },
            )?
            .join(String::from("").as_str()),
    )
}
/// Fn `todo.render.urgency`, `src/todo/render.slg:22`.
pub fn urgency(p: crate::todo::model::Priority) -> R<Int> {
    Ok(
        if let crate::todo::model::Priority::High = &p {
            0
        } else if let crate::todo::model::Priority::Normal = &p {
            1
        } else if let crate::todo::model::Priority::Low = &p {
            2
        } else {
            return Err(rt::stop("E-RUNTIME", "no arm matches"))
        },
    )
}
/// most urgent first, then oldest first
///
/// Fn `todo.render.rank`, `src/todo/render.slg:28`.
pub fn rank(t: crate::todo::model::Task) -> R<(Int, Int)> {
    rt::claim(crate::conform::task(&t)?, "argument `t` of todo.render.rank")?;
    Ok((urgency(t.priority.clone())?, t.no))
}
/// Fn `todo.render.line`, `src/todo/render.slg:31`.
pub fn line(t: crate::todo::model::Task) -> R<String> {
    rt::claim(crate::conform::task(&t)?, "argument `t` of todo.render.line")?;
    Ok(
        format!(
            "{} [{}] {}{}{}",
            t.no.show(),
            mark(t.status.clone())?,
            t.title,
            badge(t.priority.clone())?,
            tag_text(t.tags.clone())?,
        ),
    )
}
