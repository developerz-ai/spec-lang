//! Module `todo.model`, printed by spec-lang from `src/todo/model.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.
use crate::rt::{self, Int, R, Show};
use std::collections::BTreeSet;
/// Type `todo.model.Title`, `src/todo/model.slg:14`.
/// Checked against `Text where is_title(self)` wherever a value of it is given.
pub type Title = String;
/// Type `todo.model.Tag`, `src/todo/model.slg:15`.
/// Checked against `Text where is_tag(self)` wherever a value of it is given.
pub type Tag = String;
/// Type `todo.model.TaskNo`, `src/todo/model.slg:16`.
/// Checked against `Int where self >= 1` wherever a value of it is given.
pub type TaskNo = Int;
/// Sum `todo.model.Status`, `src/todo/model.slg:18`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    Open,
    Done,
}
impl Show for Status {
    fn show(&self) -> String {
        match self {
            Status::Open => "Open".to_string(),
            Status::Done => "Done".to_string(),
        }
    }
}
/// Sum `todo.model.Priority`, `src/todo/model.slg:19`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Normal,
    High,
}
impl Show for Priority {
    fn show(&self) -> String {
        match self {
            Priority::Low => "Low".to_string(),
            Priority::Normal => "Normal".to_string(),
            Priority::High => "High".to_string(),
        }
    }
}
/// Record `todo.model.Task`, `src/todo/model.slg:21`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Task {
    pub no: Int,
    pub title: String,
    pub status: Status,
    pub priority: Priority,
    pub tags: BTreeSet<String>,
}
impl Show for Task {
    fn show(&self) -> String {
        rt::show::record(
            "Task",
            &[
                ("no", self.no.show()),
                ("title", self.title.show()),
                ("status", self.status.show()),
                ("priority", self.priority.show()),
                ("tags", self.tags.show()),
            ],
        )
    }
}
impl Task {
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new(
        no: Int,
        title: String,
        status: Option<Status>,
        priority: Option<Priority>,
        tags: Option<BTreeSet<String>>,
    ) -> R<Self> {
        rt::claim(crate::conform::task_no(&no)?, "field `no`")?;
        rt::claim(crate::conform::title(&title)?, "field `title`")?;
        let status = match status {
            Some(v) => v,
            None => Status::Open,
        };
        let priority = match priority {
            Some(v) => v,
            None => Priority::Normal,
        };
        let tags = tags.unwrap_or_default();
        rt::claim(crate::conform::set_tag(&tags)?, "field `tags`")?;
        Ok(Task {
            no,
            title,
            status,
            priority,
            tags,
        })
    }
}
/// The claim `is_title(self)`.
pub fn title_holds(self_: String) -> R<bool> {
    is_title(self_.clone())
}
/// The claim `is_tag(self)`.
pub fn tag_holds(self_: String) -> R<bool> {
    is_tag(self_.clone())
}
/// The claim `self >= 1`.
pub fn task_no_holds(self_: Int) -> R<bool> {
    Ok(self_ >= 1)
}
/// a title fits on one line of a terminal list
///
/// Fn `todo.model.is_title`, `src/todo/model.slg:3`.
pub fn is_title(t: String) -> R<bool> {
    Ok(
        (((((rt::text::size(&rt::text::trim(&t)) >= 1) && (rt::text::size(&t) <= 200))
            && rt::list::all(
                &rt::text::bytes(&t),
                |it: Int| -> R<bool> { Ok((it >= 32) && (it != 127)) },
            )?) && (!t.contains(String::from("\u{85}").as_str())))
            && (!t.contains(String::from("\u{2028}").as_str())))
            && (!t.contains(String::from("\u{2029}").as_str())),
    )
}
/// a tag is typed in a shell without quotes, and never reads as a flag
///
/// Fn `todo.model.is_tag`, `src/todo/model.slg:9`.
pub fn is_tag(t: String) -> R<bool> {
    let tmp1 = 1;
    Ok(
        ({
            let tmp2 = rt::text::size(&t);
            tmp1 <= tmp2
                && {
                    let tmp3 = 32;
                    tmp2 <= tmp3
                }
        }
            && rt::list::all(
                &rt::text::bytes(&t),
                |it: Int| -> R<bool> {
                    let tmp4 = 97;
                    Ok(
                        ({
                            let tmp5 = it;
                            tmp4 <= tmp5
                                && {
                                    let tmp6 = 122;
                                    tmp5 <= tmp6
                                }
                        }
                            || {
                                let tmp7 = 48;
                                {
                                    let tmp8 = it;
                                    tmp7 <= tmp8
                                        && {
                                            let tmp9 = 57;
                                            tmp8 <= tmp9
                                        }
                                }
                            }) || (it == 45),
                    )
                },
            )?) && (!t.starts_with(String::from("-").as_str())),
    )
}
