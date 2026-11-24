//! The app's manifest: its notes, parameters and rules as data, with calls from plain data.
use crate::rt::{self, Int, Opt, R};
use crate::machine::{Machine, World};
use crate::rt::data::{AsData, Data, Shape, Ty};
use crate::rt::app::{
    App, AppParam, Decl, Note, NoteArg, NoteValue, Param, Read, Rule, State,
};
use std::collections::{BTreeMap, BTreeSet};
fn call_add_0(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<String>(&mut args, "title")?;
    let a1 = rt::app::arg_or_default::<
        crate::todo::model::Priority,
    >(&mut args, "priority")?;
    let a2 = rt::app::arg_or_default::<BTreeSet<String>>(&mut args, "tags")?;
    Ok(crate::todo::board::add(m, a0, a1, a2)?.to_data())
}
fn call_list_1(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg_or_default::<bool>(&mut args, "all")?;
    let a1 = rt::app::arg_or_default::<Opt<String>>(&mut args, "tag")?;
    Ok(crate::todo::board::list(m, a0, a1)?.to_data())
}
fn call_done_2(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<Int>(&mut args, "no")?;
    Ok(crate::todo::board::done(m, a0)?.to_data())
}
fn call_reopen_3(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<Int>(&mut args, "no")?;
    Ok(crate::todo::board::reopen(m, a0)?.to_data())
}
fn call_rename_4(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<Int>(&mut args, "no")?;
    let a1 = rt::app::arg::<String>(&mut args, "title")?;
    Ok(crate::todo::board::rename(m, a0, a1)?.to_data())
}
fn call_remove_5(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<Int>(&mut args, "no")?;
    Ok(crate::todo::board::remove(m, a0)?.to_data())
}
fn call_clear_6(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    Ok(crate::todo::board::clear(m)?.to_data())
}
fn call_stats_7(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    Ok(crate::todo::board::stats(m)?.to_data())
}
impl AsData for crate::todo::model::Status {
    fn to_data(&self) -> Data {
        match self {
            crate::todo::model::Status::Open => Data::Variant("Open", Vec::new()),
            crate::todo::model::Status::Done => Data::Variant("Done", Vec::new()),
        }
    }
    fn from_data(d: &Data) -> Option<Self> {
        let Data::Variant(name, _) = d else { return None };
        match *name {
            "Open" => Some(crate::todo::model::Status::Open),
            "Done" => Some(crate::todo::model::Status::Done),
            _ => None,
        }
    }
}
impl AsData for crate::todo::model::Priority {
    fn to_data(&self) -> Data {
        match self {
            crate::todo::model::Priority::Low => Data::Variant("Low", Vec::new()),
            crate::todo::model::Priority::Normal => Data::Variant("Normal", Vec::new()),
            crate::todo::model::Priority::High => Data::Variant("High", Vec::new()),
        }
    }
    fn from_data(d: &Data) -> Option<Self> {
        let Data::Variant(name, _) = d else { return None };
        match *name {
            "Low" => Some(crate::todo::model::Priority::Low),
            "Normal" => Some(crate::todo::model::Priority::Normal),
            "High" => Some(crate::todo::model::Priority::High),
            _ => None,
        }
    }
}
impl AsData for crate::todo::model::Task {
    fn to_data(&self) -> Data {
        Data::Record(
            vec![
                ("no", self.no.to_data()),
                ("title", self.title.to_data()),
                ("status", self.status.to_data()),
                ("priority", self.priority.to_data()),
                ("tags", self.tags.to_data()),
            ],
        )
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::todo::model::Task {
            no: AsData::from_data(d.field("no")?)?,
            title: AsData::from_data(d.field("title")?)?,
            status: AsData::from_data(d.field("status")?)?,
            priority: AsData::from_data(d.field("priority")?)?,
            tags: AsData::from_data(d.field("tags")?)?,
        })
    }
}
/// The world as data: every state by name.
fn world_data(w: &World) -> Data {
    Data::Record(vec![("tasks", w.tasks.to_data()), ("next_no", w.next_no.to_data())])
}
/// A world from data, when every state is of its Rust type (claims are checked apart).
fn data_world(d: &Data) -> Option<World> {
    Some(World {
        tasks: AsData::from_data(d.field("tasks")?)?,
        next_no: AsData::from_data(d.field("next_no")?)?,
    })
}
/// The app `apps.rust.TodoRust`: what its libraries' runtimes read.
pub static APP: App = App {
    name: "TodoRust",
    qualified: "apps.rust.TodoRust",
    module: "apps.rust",
    target: Some("rust"),
    shape: None,
    notes: &[
        Note {
            library: "cli",
            name: "bin",
            args: &[
                NoteArg {
                    param: "name",
                    value: NoteValue::Text("todo"),
                },
            ],
        },
        Note {
            library: "cli",
            name: "store",
            args: &[
                NoteArg {
                    param: "at",
                    value: NoteValue::Variant(
                        "file",
                        &[
                            NoteArg {
                                param: "path",
                                value: NoteValue::Text(".todo.json"),
                            },
                        ],
                    ),
                },
            ],
        },
    ],
    params: &[
        AppParam {
            name: "rules",
            decls: &[
                Decl::Rule(0),
                Decl::Rule(1),
                Decl::Rule(2),
                Decl::Rule(3),
                Decl::Rule(4),
                Decl::Rule(5),
                Decl::Rule(6),
                Decl::Rule(7),
            ],
        },
    ],
    rules: &[
        Rule {
            name: "add",
            qualified: "todo.board.add",
            params: &[
                Param {
                    name: "title",
                    ty: &TY_TITLE,
                    has_default: false,
                },
                Param {
                    name: "priority",
                    ty: &TY_PRIORITY,
                    has_default: true,
                },
                Param {
                    name: "tags",
                    ty: &TY_SET_TAG,
                    has_default: true,
                },
            ],
            result: &TY_TASK_NO,
            triggers: &[
                Note {
                    library: "cli",
                    name: "cli",
                    args: &[
                        NoteArg {
                            param: "command",
                            value: NoteValue::Text("add"),
                        },
                    ],
                },
            ],
            uses: &["tasks.write", "next_no.write", "out.write"],
            writes: true,
            call: call_add_0,
        },
        Rule {
            name: "list",
            qualified: "todo.board.list",
            params: &[
                Param {
                    name: "all",
                    ty: &TY_BOOL,
                    has_default: true,
                },
                Param {
                    name: "tag",
                    ty: &TY_OPTION_TAG,
                    has_default: true,
                },
            ],
            result: &TY_UNIT,
            triggers: &[
                Note {
                    library: "cli",
                    name: "cli",
                    args: &[
                        NoteArg {
                            param: "command",
                            value: NoteValue::Text("list"),
                        },
                    ],
                },
            ],
            uses: &["tasks.read", "out.write"],
            writes: false,
            call: call_list_1,
        },
        Rule {
            name: "done",
            qualified: "todo.board.done",
            params: &[
                Param {
                    name: "no",
                    ty: &TY_TASK_NO,
                    has_default: false,
                },
            ],
            result: &TY_UNIT,
            triggers: &[
                Note {
                    library: "cli",
                    name: "cli",
                    args: &[
                        NoteArg {
                            param: "command",
                            value: NoteValue::Text("done"),
                        },
                    ],
                },
            ],
            uses: &["tasks.write", "out.write"],
            writes: true,
            call: call_done_2,
        },
        Rule {
            name: "reopen",
            qualified: "todo.board.reopen",
            params: &[
                Param {
                    name: "no",
                    ty: &TY_TASK_NO,
                    has_default: false,
                },
            ],
            result: &TY_UNIT,
            triggers: &[
                Note {
                    library: "cli",
                    name: "cli",
                    args: &[
                        NoteArg {
                            param: "command",
                            value: NoteValue::Text("reopen"),
                        },
                    ],
                },
            ],
            uses: &["tasks.write", "out.write"],
            writes: true,
            call: call_reopen_3,
        },
        Rule {
            name: "rename",
            qualified: "todo.board.rename",
            params: &[
                Param {
                    name: "no",
                    ty: &TY_TASK_NO,
                    has_default: false,
                },
                Param {
                    name: "title",
                    ty: &TY_TITLE,
                    has_default: false,
                },
            ],
            result: &TY_UNIT,
            triggers: &[
                Note {
                    library: "cli",
                    name: "cli",
                    args: &[
                        NoteArg {
                            param: "command",
                            value: NoteValue::Text("rename"),
                        },
                    ],
                },
            ],
            uses: &["tasks.write", "out.write"],
            writes: true,
            call: call_rename_4,
        },
        Rule {
            name: "remove",
            qualified: "todo.board.remove",
            params: &[
                Param {
                    name: "no",
                    ty: &TY_TASK_NO,
                    has_default: false,
                },
            ],
            result: &TY_UNIT,
            triggers: &[
                Note {
                    library: "cli",
                    name: "cli",
                    args: &[
                        NoteArg {
                            param: "command",
                            value: NoteValue::Text("remove"),
                        },
                    ],
                },
            ],
            uses: &["tasks.write", "out.write"],
            writes: true,
            call: call_remove_5,
        },
        Rule {
            name: "clear",
            qualified: "todo.board.clear",
            params: &[],
            result: &TY_INT,
            triggers: &[
                Note {
                    library: "cli",
                    name: "cli",
                    args: &[
                        NoteArg {
                            param: "command",
                            value: NoteValue::Text("clear"),
                        },
                    ],
                },
            ],
            uses: &["tasks.write", "out.write"],
            writes: true,
            call: call_clear_6,
        },
        Rule {
            name: "stats",
            qualified: "todo.board.stats",
            params: &[],
            result: &TY_UNIT,
            triggers: &[
                Note {
                    library: "cli",
                    name: "cli",
                    args: &[
                        NoteArg {
                            param: "command",
                            value: NoteValue::Text("stats"),
                        },
                    ],
                },
            ],
            uses: &["tasks.read", "out.write"],
            writes: false,
            call: call_stats_7,
        },
    ],
    fns: &[],
    states: &[
        State {
            name: "tasks",
            qualified: "todo.board.tasks",
            ty: &TY_MAP_TASK_NO_TASK,
        },
        State {
            name: "next_no",
            qualified: "todo.board.next_no",
            ty: &TY_TASK_NO,
        },
    ],
    reads: &[
        Read {
            library: "cli",
            handler: "out",
            op: "text",
            read: |m| Ok(crate::machine::out_text(m)?.to_data()),
        },
    ],
    world_data,
    data_world,
    lawful: rt::runner::lawful,
};
/// `todo.model.Title`.
pub static TY_TITLE: Ty = Ty {
    name: "Title",
    shape: Shape::Text,
    admits: |d| rt::data::admits::<
        String,
    >(d, |v| crate::conform::title(v).unwrap_or(false)),
};
/// `todo.model.Priority`.
pub static TY_PRIORITY: Ty = Ty {
    name: "Priority",
    shape: Shape::Sum(&[("Low", &[]), ("Normal", &[]), ("High", &[])]),
    admits: |d| rt::data::admits::<crate::todo::model::Priority>(d, |_| true),
};
/// `Set[todo.model.Tag]`.
pub static TY_SET_TAG: Ty = Ty {
    name: "Set[todo.model.Tag]",
    shape: Shape::Set(&TY_TAG),
    admits: |d| rt::data::admits::<
        BTreeSet<String>,
    >(d, |v| crate::conform::set_tag(v).unwrap_or(false)),
};
/// `todo.model.TaskNo`.
pub static TY_TASK_NO: Ty = Ty {
    name: "TaskNo",
    shape: Shape::Int,
    admits: |d| rt::data::admits::<
        Int,
    >(d, |v| crate::conform::task_no(v).unwrap_or(false)),
};
/// `Bool`.
pub static TY_BOOL: Ty = Ty {
    name: "Bool",
    shape: Shape::Bool,
    admits: |d| rt::data::admits::<bool>(d, |_| true),
};
/// `Option[todo.model.Tag]`.
pub static TY_OPTION_TAG: Ty = Ty {
    name: "Option[todo.model.Tag]",
    shape: Shape::Option(&TY_TAG),
    admits: |d| rt::data::admits::<
        Opt<String>,
    >(d, |v| crate::conform::option_tag(v).unwrap_or(false)),
};
/// `()`.
pub static TY_UNIT: Ty = Ty {
    name: "()",
    shape: Shape::Unit,
    admits: |d| rt::data::admits::<()>(d, |_| true),
};
/// `Int`.
pub static TY_INT: Ty = Ty {
    name: "Int",
    shape: Shape::Int,
    admits: |d| rt::data::admits::<Int>(d, |_| true),
};
/// `Map[todo.model.TaskNo, todo.model.Task]`.
pub static TY_MAP_TASK_NO_TASK: Ty = Ty {
    name: "Map[todo.model.TaskNo, todo.model.Task]",
    shape: Shape::Map(&TY_TASK_NO, &TY_TASK),
    admits: |d| rt::data::admits::<
        BTreeMap<Int, crate::todo::model::Task>,
    >(d, |v| crate::conform::map_task_no_task(v).unwrap_or(false)),
};
/// `todo.model.Tag`.
pub static TY_TAG: Ty = Ty {
    name: "Tag",
    shape: Shape::Text,
    admits: |d| rt::data::admits::<
        String,
    >(d, |v| crate::conform::tag(v).unwrap_or(false)),
};
/// `todo.model.Task`.
pub static TY_TASK: Ty = Ty {
    name: "Task",
    shape: Shape::Record(
        &[
            ("no", &TY_TASK_NO),
            ("title", &TY_TITLE),
            ("status", &TY_STATUS),
            ("priority", &TY_PRIORITY),
            ("tags", &TY_SET_TAG),
        ],
    ),
    admits: |d| rt::data::admits::<
        crate::todo::model::Task,
    >(d, |v| crate::conform::task(v).unwrap_or(false)),
};
/// `todo.model.Status`.
pub static TY_STATUS: Ty = Ty {
    name: "Status",
    shape: Shape::Sum(&[("Open", &[]), ("Done", &[])]),
    admits: |d| rt::data::admits::<crate::todo::model::Status>(d, |_| true),
};
