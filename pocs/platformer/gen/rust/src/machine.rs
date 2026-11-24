//! The world, the machine, each handler operation, events and replies of traces.

use crate::rt::{self, Failure, Int, R, Show};
use crate::rt::rng::Rng;
use std::collections::{BTreeMap, BTreeSet};

/// Every state's value at one moment.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct World {
    /// `hop.world.stage`
    pub stage: crate::hop::model::Level,
    /// `hop.world.ticks`
    pub ticks: Int,
    /// `hop.world.player`
    pub player: crate::hop::model::Player,
    /// `hop.world.taken`
    pub taken: BTreeSet<Int>,
    /// `hop.world.crawlers`
    pub crawlers: BTreeMap<Int, crate::hop::model::Crawler>,
    /// `hop.world.lives`
    pub lives: Int,
    /// `hop.world.mode`
    pub mode: crate::hop::model::Mode,
}

impl Show for World {
    fn show(&self) -> String {
        rt::show::seq(
            "World(",
            [
                format!("hop.world.stage: {}", self.stage.show()),
                format!("hop.world.ticks: {}", self.ticks.show()),
                format!("hop.world.player: {}", self.player.show()),
                format!("hop.world.taken: {}", self.taken.show()),
                format!("hop.world.crawlers: {}", self.crawlers.show()),
                format!("hop.world.lives: {}", self.lives.show()),
                format!("hop.world.mode: {}", self.mode.show()),
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
        let stage = crate::hop::levels::meadow()?;
        let ticks = 0;
        let player = crate::hop::model::Player::new(
            crate::hop::levels::meadow()?.start,
            None,
            None,
            None,
            None,
            None,
            None,
        )?;
        let taken = BTreeSet::new();
        let crawlers = BTreeMap::new();
        let lives = 3;
        let mode = crate::hop::model::Mode::Playing;
        rt::claim(
            crate::conform::level(&stage)?,
            "the initial value of hop.world.stage",
        )?;
        rt::claim(
            crate::conform::player(&player)?,
            "the initial value of hop.world.player",
        )?;
        rt::claim(
            crate::conform::map_int_crawler(&crawlers)?,
            "the initial value of hop.world.crawlers",
        )?;
        rt::claim(
            crate::conform::range_0_3(&lives)?,
            "the initial value of hop.world.lives",
        )?;
        Ok(Machine {
            world: World {
                stage,
                ticks,
                player,
                taken,
                crawlers,
                lives,
                mode,
            },
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
    /// `hop.screen.clear_background`
    ClearBackground { color: crate::hop::model::Rgb },
    /// `hop.screen.begin_mode_3d`
    BeginMode3d { eye: crate::hop::geom::Vec3, target: crate::hop::geom::Vec3 },
    /// `hop.screen.end_mode_3d`
    EndMode3d {},
    /// `hop.screen.begin_lit`
    BeginLit { time: Int },
    /// `hop.screen.end_lit`
    EndLit {},
    /// `hop.screen.draw_cube`
    DrawCube { r#box: crate::hop::geom::Box, color: crate::hop::model::Rgb },
    /// `hop.screen.draw_cube_wires`
    DrawCubeWires { r#box: crate::hop::geom::Box, color: crate::hop::model::Rgb },
    /// `hop.screen.draw_model`
    DrawModel { asset: crate::hop::render::Asset, at: crate::hop::geom::Vec3, yaw: Int },
    /// `hop.screen.draw_text`
    DrawText { text: String, x: Int, y: Int, size: Int, color: crate::hop::model::Rgb },
    /// `hop.screen.present`
    Present {},
    /// `hop.world.start`
    Start { level: Int },
    /// `hop.world.lose`
    Lose { why: String },
    /// `hop.world.step`
    Step { pad: crate::hop::model::Pad },
}

impl Show for Event {
    fn show(&self) -> String {
        match self {
            Event::ClearBackground { color } => {
                rt::show::seq(
                    "screen.clear_background(",
                    [color.show()].into_iter(),
                    ")",
                )
            }
            Event::BeginMode3d { eye, target } => {
                rt::show::seq(
                    "screen.begin_mode_3d(",
                    [eye.show(), format!("target: {}", target.show())].into_iter(),
                    ")",
                )
            }
            Event::EndMode3d {} => {
                rt::show::seq("screen.end_mode_3d(", [].into_iter(), ")")
            }
            Event::BeginLit { time } => {
                rt::show::seq("screen.begin_lit(", [time.show()].into_iter(), ")")
            }
            Event::EndLit {} => rt::show::seq("screen.end_lit(", [].into_iter(), ")"),
            Event::DrawCube { r#box, color } => {
                rt::show::seq(
                    "screen.draw_cube(",
                    [r#box.show(), format!("color: {}", color.show())].into_iter(),
                    ")",
                )
            }
            Event::DrawCubeWires { r#box, color } => {
                rt::show::seq(
                    "screen.draw_cube_wires(",
                    [r#box.show(), format!("color: {}", color.show())].into_iter(),
                    ")",
                )
            }
            Event::DrawModel { asset, at, yaw } => {
                rt::show::seq(
                    "screen.draw_model(",
                    [
                        asset.show(),
                        format!("at: {}", at.show()),
                        format!("yaw: {}", yaw.show()),
                    ]
                        .into_iter(),
                    ")",
                )
            }
            Event::DrawText { text, x, y, size, color } => {
                rt::show::seq(
                    "screen.draw_text(",
                    [
                        text.show(),
                        format!("x: {}", x.show()),
                        format!("y: {}", y.show()),
                        format!("size: {}", size.show()),
                        format!("color: {}", color.show()),
                    ]
                        .into_iter(),
                    ")",
                )
            }
            Event::Present {} => rt::show::seq("screen.present(", [].into_iter(), ")"),
            Event::Start { level } => {
                rt::show::seq("world.start(", [level.show()].into_iter(), ")")
            }
            Event::Lose { why } => {
                rt::show::seq("world.lose(", [why.show()].into_iter(), ")")
            }
            Event::Step { pad } => {
                rt::show::seq("world.step(", [pad.show()].into_iter(), ")")
            }
        }
    }
}

impl Event {
    /// A call of rule number `rule` with arguments drawn from its parameters' types.
    pub fn random(
        m: &mut Machine,
        rng: &mut Rng,
        pools: &Pools,
        rule: usize,
    ) -> R<Event> {
        Ok(
            match rule {
                10 => {
                    Event::Start {
                        level: crate::draw::level_no(m, rng, pools)?,
                    }
                }
                11 => {
                    Event::Lose {
                        why: crate::draw::text(m, rng, pools)?,
                    }
                }
                12 => {
                    Event::Step {
                        pad: crate::draw::pad(m, rng, pools)?,
                    }
                }
                _ => return Err(rt::stop("E-GEN", "no such rule")),
            },
        )
    }
    /// The call as one step from outside: its value or its refusal.
    pub fn call(&self, m: &mut Machine) -> R<Reply> {
        Ok(
            match self {
                Event::ClearBackground { color } => {
                    match rt::settle(
                        crate::hop::screen::clear_background(m, color.clone()),
                    )? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::BeginMode3d { eye, target } => {
                    match rt::settle(
                        crate::hop::screen::begin_mode_3d(m, eye.clone(), target.clone()),
                    )? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::EndMode3d {} => {
                    match rt::settle(crate::hop::screen::end_mode_3d(m))? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::BeginLit { time } => {
                    match rt::settle(crate::hop::screen::begin_lit(m, *time))? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::EndLit {} => {
                    match rt::settle(crate::hop::screen::end_lit(m))? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::DrawCube { r#box, color } => {
                    match rt::settle(
                        crate::hop::screen::draw_cube(m, r#box.clone(), color.clone()),
                    )? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::DrawCubeWires { r#box, color } => {
                    match rt::settle(
                        crate::hop::screen::draw_cube_wires(
                            m,
                            r#box.clone(),
                            color.clone(),
                        ),
                    )? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::DrawModel { asset, at, yaw } => {
                    match rt::settle(
                        crate::hop::screen::draw_model(
                            m,
                            asset.clone(),
                            at.clone(),
                            *yaw,
                        ),
                    )? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::DrawText { text, x, y, size, color } => {
                    match rt::settle(
                        crate::hop::screen::draw_text(
                            m,
                            text.clone(),
                            *x,
                            *y,
                            *size,
                            color.clone(),
                        ),
                    )? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::Present {} => {
                    match rt::settle(crate::hop::screen::present(m))? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::Start { level } => {
                    match rt::settle(crate::hop::world::start(m, *level))? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::Lose { why } => {
                    match rt::settle(crate::hop::world::lose(m, why.clone()))? {
                        Ok(v) => Reply::Ok(Out::Unit(v)),
                        Err(f) => Reply::Err(f),
                    }
                }
                Event::Step { pad } => {
                    match rt::settle(crate::hop::world::step(m, pad.clone()))? {
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
}

impl Show for Out {
    fn show(&self) -> String {
        match self {
            Out::Unit(v) => v.show(),
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
    pub fn of(worlds: &[World]) -> Pools {
        let mut p0 = BTreeSet::new();
        for w in worlds {
            p0.extend(w.taken.iter().cloned());
            p0.extend(w.crawlers.keys().cloned());
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
        stage: crate::draw::level(m, rng, &pools)?,
        ticks: crate::draw::int(m, rng, &pools)?,
        player: crate::draw::player(m, rng, &pools)?,
        taken: crate::draw::set_int(m, rng, &pools)?,
        crawlers: crate::draw::map_int_crawler(m, rng, &pools)?,
        lives: crate::draw::range_0_3(m, rng, &pools)?,
        mode: crate::draw::mode(m, rng, &pools)?,
    })
}
