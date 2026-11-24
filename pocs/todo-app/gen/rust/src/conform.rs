//! Whether a value is of a declared type: ranges and `where` claims, field by field.

use crate::rt::{self, Int, Opt, R};
use std::collections::{BTreeMap, BTreeSet};

/// Whether a value is of `todo.model.Task`.
pub fn task(v: &crate::todo::model::Task) -> R<bool> {
    if !task_no(&v.no)? {
        return Ok(false);
    }
    if !title(&v.title)? {
        return Ok(false);
    }
    set_tag(&v.tags)
}

/// Whether a value is of `Option[todo.model.Tag]`.
pub fn option_tag(v: &Opt<String>) -> R<bool> {
    if let Opt::Some(x) = v && !tag(x)? {
        return Ok(false);
    }
    Ok(true)
}

/// Whether a value is of `todo.model.Title`.
pub fn title(v: &str) -> R<bool> {
    text_where0(v)
}

/// Whether a value is of `Set[todo.model.Tag]`.
pub fn set_tag(v: &BTreeSet<String>) -> R<bool> {
    for x in v {
        if !tag(x)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether a value is of `todo.model.TaskNo`.
pub fn task_no(v: &Int) -> R<bool> {
    int_where2(v)
}

/// Whether a value is of `Map[todo.model.TaskNo, todo.model.Task]`.
pub fn map_task_no_task(v: &BTreeMap<Int, crate::todo::model::Task>) -> R<bool> {
    for x in v.keys() {
        if !task_no(x)? {
            return Ok(false);
        }
    }
    for x in v.values() {
        if !task(x)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Whether a value is of `Text where is_title(self)`.
pub fn text_where0(v: &str) -> R<bool> {
    rt::holds(crate::todo::model::title_holds(v.to_owned()))
}

/// Whether a value is of `todo.model.Tag`.
pub fn tag(v: &str) -> R<bool> {
    text_where1(v)
}

/// Whether a value is of `Int where self >= 1`.
pub fn int_where2(v: &Int) -> R<bool> {
    rt::holds(crate::todo::model::task_no_holds(*v))
}

/// Whether a value is of `Text where is_tag(self)`.
pub fn text_where1(v: &str) -> R<bool> {
    rt::holds(crate::todo::model::tag_holds(v.to_owned()))
}
