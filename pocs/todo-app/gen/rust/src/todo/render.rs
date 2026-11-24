//! Module `todo.render`, printed by spec-lang from `src/todo/render.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.

use crate::rt::{self, Int, R, Show};
use std::collections::BTreeSet;

/// Fn `todo.render.mark`, `src/todo/render.slg:8`.
pub fn mark(s: crate::todo::model::Status) -> R<String> {
    Ok(
        match &s {
            crate::todo::model::Status::Open => String::from(" "),
            crate::todo::model::Status::Done => String::from("x"),
        },
    )
}

/// Fn `todo.render.badge`, `src/todo/render.slg:13`.
pub fn badge(p: crate::todo::model::Priority) -> R<String> {
    Ok(
        match &p {
            crate::todo::model::Priority::Low => String::from(" !low"),
            crate::todo::model::Priority::Normal => String::from(""),
            crate::todo::model::Priority::High => String::from(" !high"),
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
        match &p {
            crate::todo::model::Priority::High => 0,
            crate::todo::model::Priority::Normal => 1,
            crate::todo::model::Priority::Low => 2,
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
