//! The world, the machine, each handler operation, events and replies of traces.
use crate::rt::{self, Failure, Int, Opt, R, Show};
use crate::rt::rng::Rng;
use std::collections::{BTreeMap, BTreeSet};
/// Every state's value at one moment.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct World {
    /// `todo.board.tasks`
    pub tasks: BTreeMap<Int, crate::todo::model::Task>,
    /// `todo.board.next_no`
    pub next_no: Int,
}
impl Show for World {
    fn show(&self) -> String {
        rt::show::seq(
            "World(",
            [
                format!("todo.board.tasks: {}", self.tasks.show()),
                format!("todo.board.next_no: {}", self.next_no.show()),
            ]
                .into_iter(),
            ")",
        )
    }
}
/// The state of `PretendOut`, what tests run for `out`.
#[derive(Clone, Debug)]
pub struct PretendOut {
    pub sent: Vec<String>,
}
/// Every handler's pretend state.
#[derive(Clone, Debug)]
pub struct Handlers {
    pub out: PretendOut,
}
/// One run of the program: the world, the handlers' state.
#[derive(Clone, Debug)]
pub struct Machine {
    pub world: World,
    pub handlers: Handlers,
}
impl Machine {
    /// The initial world: every state's initial value, checked against its type.
    pub fn new() -> R<Machine> {
        let tasks = BTreeMap::new();
        let next_no = 1;
        rt::claim(
            crate::conform::map_task_no_task(&tasks)?,
            "the initial value of todo.board.tasks",
        )?;
        rt::claim(
            crate::conform::task_no(&next_no)?,
            "the initial value of todo.board.next_no",
        )?;
        Ok(Machine {
            world: World { tasks, next_no },
            handlers: Handlers {
                out: PretendOut {
                    sent: Vec::<String>::new(),
                },
            },
        })
    }
    /// One atomic step: on any halt, the world and the handlers' state go back to what they were.
    pub fn atomic<T>(&mut self, step: impl FnOnce(&mut Machine) -> R<T>) -> R<T> {
        let saved = (self.world.clone(), self.handlers.clone());
        let result = step(self);
        if result.is_err() {
            (self.world, self.handlers) = saved;
        }
        result
    }
}
/// `out.line` of library `cli`: in checks, its pretend runs.
pub fn out_line(m: &mut Machine, text: String) -> R<()> {
    let mut sent = m.handlers.out.sent.clone();
    let reply: () = {
        let tmp1 = text.clone();
        sent.push(tmp1);
    };
    m.handlers.out.sent = sent;
    Ok(reply)
}
/// `out.text` of library `cli`: in checks, its pretend runs.
pub fn out_text(m: &mut Machine) -> R<String> {
    let sent = m.handlers.out.sent.clone();
    let reply: String = rt::list::map(
            &sent,
            |line: String| -> R<String> { Ok(format!("{}\n", line)) },
        )?
        .join(String::from("").as_str());
    Ok(reply)
}
/// A call of a rule with its arguments, as a trace records it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Event {
    /// `todo.board.add`
    Add {
        title: String,
        priority: crate::todo::model::Priority,
        tags: BTreeSet<String>,
    },
    /// `todo.board.list`
    List { all: bool, tag: Opt<String> },
    /// `todo.board.done`
    Done { no: Int },
    /// `todo.board.reopen`
    Reopen { no: Int },
    /// `todo.board.rename`
    Rename { no: Int, title: String },
    /// `todo.board.remove`
    Remove { no: Int },
    /// `todo.board.clear`
    Clear {},
    /// `todo.board.stats`
    Stats {},
}
impl Show for Event {
    fn show(&self) -> String {
        match self {
            Event::Add { title, priority, tags } => {
                rt::show::seq(
                    "board.add(",
                    [
                        title.show(),
                        format!("priority: {}", priority.show()),
                        format!("tags: {}", tags.show()),
                    ]
                        .into_iter(),
                    ")",
                )
            }
            Event::List { all, tag } => {
                rt::show::seq(
                    "board.list(",
                    [all.show(), format!("tag: {}", tag.show())].into_iter(),
                    ")",
                )
            }
            Event::Done { no } => {
                rt::show::seq("board.done(", [no.show()].into_iter(), ")")
            }
            Event::Reopen { no } => {
                rt::show::seq("board.reopen(", [no.show()].into_iter(), ")")
            }
            Event::Rename { no, title } => {
                rt::show::seq(
                    "board.rename(",
                    [no.show(), format!("title: {}", title.show())].into_iter(),
                    ")",
                )
            }
            Event::Remove { no } => {
                rt::show::seq("board.remove(", [no.show()].into_iter(), ")")
            }
            Event::Clear {} => rt::show::seq("board.clear(", [].into_iter(), ")"),
            Event::Stats {} => rt::show::seq("board.stats(", [].into_iter(), ")"),
        }
    }
}
impl Event {
    /// A call of rule number `rule` with arguments drawn from its parameters' types.
    #[allow(unused_variables)]
    pub fn random(
        m: &mut Machine,
        rng: &mut Rng,
        pools: &Pools,
        rule: usize,
    ) -> R<Event> {
        Ok(
            match rule {
                0 => {
                    Event::Add {
                        title: crate::draw::title(m, rng, pools)?,
                        priority: crate::draw::priority(m, rng, pools)?,
                        tags: crate::draw::set_tag(m, rng, pools)?,
                    }
                }
                1 => {
                    Event::List {
                        all: crate::draw::bool(m, rng, pools)?,
                        tag: crate::draw::option_tag(m, rng, pools)?,
                    }
                }
                2 => {
                    Event::Done {
                        no: crate::draw::task_no(m, rng, pools)?,
                    }
                }
                3 => {
                    Event::Reopen {
                        no: crate::draw::task_no(m, rng, pools)?,
                    }
                }
                4 => {
                    Event::Rename {
                        no: crate::draw::task_no(m, rng, pools)?,
                        title: crate::draw::title(m, rng, pools)?,
                    }
                }
                5 => {
                    Event::Remove {
                        no: crate::draw::task_no(m, rng, pools)?,
                    }
                }
                6 => Event::Clear {},
                7 => Event::Stats {},
                _ => return Err(rt::stop("E-GEN", "no such rule")),
            },
        )
    }
    /// The call as one step from outside: its value or its refusal.
    pub fn call(&self, m: &mut Machine) -> R<Reply> {
        Ok(
            match self {
                Event::Add { title, priority, tags } => {
                    match rt::settle(
                        crate::todo::board::add(
                            m,
                            title.clone(),
                            Some(priority.clone()),
                            Some(tags.clone()),
                        ),
                    )? {
                        Ok(v) => Reply::Ok(Out::Int(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::List { all, tag } => {
                    match rt::settle(
                        crate::todo::board::list(m, Some(*all), Some(tag.clone())),
                    )? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::Done { no } => {
                    match rt::settle(crate::todo::board::done(m, *no))? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::Reopen { no } => {
                    match rt::settle(crate::todo::board::reopen(m, *no))? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::Rename { no, title } => {
                    match rt::settle(
                        crate::todo::board::rename(m, *no, title.clone()),
                    )? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::Remove { no } => {
                    match rt::settle(crate::todo::board::remove(m, *no))? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::Clear {} => {
                    match rt::settle(crate::todo::board::clear(m))? {
                        Ok(v) => Reply::Ok(Out::Int(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::Stats {} => {
                    match rt::settle(crate::todo::board::stats(m))? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
            },
        )
    }
}
/// What a rule gives, one variant per result type.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Out {
    Unit(()),
    Int(Int),
}
impl Show for Out {
    fn show(&self) -> String {
        match self {
            Out::Unit(v) => v.show(),
            Out::Int(v) => v.show(),
        }
    }
}
/// A step's reply: the rule's value or its refusal.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Reply {
    Ok(Out),
    Err(Failure),
}
impl Show for Reply {
    fn show(&self) -> String {
        match self {
            Reply::Ok(v) => format!("Ok({})", v.show()),
            Reply::Err(f) => format!("Err({})", f.show()),
        }
    }
}
/// Values already in a world, by declared type: the keys of each state declared `Map[K, V]` and the elements of each `Set[K]`.
#[derive(Clone, Debug, Default)]
pub struct Pools {
    pub p0: Vec<Int>,
}
impl Pools {
    #[allow(unused_variables)]
    pub fn of(worlds: &[World]) -> Pools {
        let mut p0 = BTreeSet::new();
        for w in worlds {
            p0.extend(w.tasks.keys().cloned());
        }
        Pools {
            p0: p0.into_iter().collect(),
        }
    }
}
/// A world drawn from the states' declared types, state by state.
pub fn random_world(m: &mut Machine, rng: &mut Rng) -> R<World> {
    let pools = Pools::default();
    Ok(World {
        tasks: crate::draw::map_task_no_task(m, rng, &pools)?,
        next_no: crate::draw::task_no(m, rng, &pools)?,
    })
}
