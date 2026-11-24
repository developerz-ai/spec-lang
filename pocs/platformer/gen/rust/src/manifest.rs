//! The app's manifest: its notes, parameters and rules as data, with calls from plain data.

use crate::rt::{self, Int, Opt, R};
use crate::machine::{Machine, World};
use crate::rt::data::{AsData, Data, Shape, Ty};
use crate::rt::app::{
    App, AppParam, Decl, Extern, Func, Note, NoteArg, NoteValue, Param, Read, Rule, State,
};

use std::collections::{BTreeMap, BTreeSet};

fn call_clear_background_0(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<crate::hop::model::Rgb>(&mut args, "color")?;
    Ok(crate::hop::screen::clear_background(m, a0)?.to_data())
}

fn call_begin_mode_3d_1(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<crate::hop::geom::Vec3>(&mut args, "eye")?;
    let a1 = rt::app::arg::<crate::hop::geom::Vec3>(&mut args, "target")?;
    Ok(crate::hop::screen::begin_mode_3d(m, a0, a1)?.to_data())
}

fn call_end_mode_3d_2(m: &mut Machine, _args: Vec<Option<Data>>) -> R<Data> {
    Ok(crate::hop::screen::end_mode_3d(m)?.to_data())
}

fn call_begin_lit_3(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<Int>(&mut args, "time")?;
    Ok(crate::hop::screen::begin_lit(m, a0)?.to_data())
}

fn call_end_lit_4(m: &mut Machine, _args: Vec<Option<Data>>) -> R<Data> {
    Ok(crate::hop::screen::end_lit(m)?.to_data())
}

fn call_draw_cube_5(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<crate::hop::geom::Box>(&mut args, "box")?;
    let a1 = rt::app::arg::<crate::hop::model::Rgb>(&mut args, "color")?;
    Ok(crate::hop::screen::draw_cube(m, a0, a1)?.to_data())
}

fn call_draw_cube_wires_6(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<crate::hop::geom::Box>(&mut args, "box")?;
    let a1 = rt::app::arg::<crate::hop::model::Rgb>(&mut args, "color")?;
    Ok(crate::hop::screen::draw_cube_wires(m, a0, a1)?.to_data())
}

fn call_draw_model_7(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<crate::hop::render::Asset>(&mut args, "asset")?;
    let a1 = rt::app::arg::<crate::hop::geom::Vec3>(&mut args, "at")?;
    let a2 = rt::app::arg::<Int>(&mut args, "yaw")?;
    Ok(crate::hop::screen::draw_model(m, a0, a1, a2)?.to_data())
}

fn call_draw_text_8(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<String>(&mut args, "text")?;
    let a1 = rt::app::arg::<Int>(&mut args, "x")?;
    let a2 = rt::app::arg::<Int>(&mut args, "y")?;
    let a3 = rt::app::arg::<Int>(&mut args, "size")?;
    let a4 = rt::app::arg::<crate::hop::model::Rgb>(&mut args, "color")?;
    Ok(crate::hop::screen::draw_text(m, a0, a1, a2, a3, a4)?.to_data())
}

fn call_present_9(m: &mut Machine, _args: Vec<Option<Data>>) -> R<Data> {
    Ok(crate::hop::screen::present(m)?.to_data())
}

fn call_start_10(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<Int>(&mut args, "level")?;
    Ok(crate::hop::world::start(m, a0)?.to_data())
}

fn call_lose_11(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<String>(&mut args, "why")?;
    Ok(crate::hop::world::lose(m, a0)?.to_data())
}

fn call_step_12(m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<crate::hop::model::Pad>(&mut args, "pad")?;
    Ok(crate::hop::world::step(m, a0)?.to_data())
}

fn fn_read_pad_0(_m: &mut Machine, _args: Vec<Option<Data>>) -> R<Data> {
    Ok(crate::hop::screen::read_pad()?.to_data())
}

fn fn_dump_1(m: &mut Machine, _args: Vec<Option<Data>>) -> R<Data> {
    Ok(crate::hop::render::dump(m)?.to_data())
}

fn fn_summary_2(m: &mut Machine, _args: Vec<Option<Data>>) -> R<Data> {
    Ok(crate::hop::world::summary(m)?.to_data())
}

fn fn_vec_text_3(_m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<crate::hop::geom::Vec3>(&mut args, "v")?;
    Ok(crate::hop::geom::vec_text(a0)?.to_data())
}

fn fn_body_4(_m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<crate::hop::geom::Vec3>(&mut args, "at")?;
    Ok(crate::hop::physics::body(a0)?.to_data())
}

fn fn_asset_name_5(_m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<crate::hop::render::Asset>(&mut args, "a")?;
    Ok(crate::hop::render::asset_name(a0)?.to_data())
}

fn fn_rgb_text_6(_m: &mut Machine, args: Vec<Option<Data>>) -> R<Data> {
    let mut args = args.into_iter();
    let a0 = rt::app::arg::<crate::hop::model::Rgb>(&mut args, "c")?;
    Ok(crate::hop::render::rgb_text(a0)?.to_data())
}

impl AsData for crate::hop::geom::Vec3 {
    fn to_data(&self) -> Data {
        Data::Record(
            vec![
                ("x", self.x.to_data()),
                ("y", self.y.to_data()),
                ("z", self.z.to_data()),
            ],
        )
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::hop::geom::Vec3 {
            x: AsData::from_data(d.field("x")?)?,
            y: AsData::from_data(d.field("y")?)?,
            z: AsData::from_data(d.field("z")?)?,
        })
    }
}

impl AsData for crate::hop::geom::Box {
    fn to_data(&self) -> Data {
        Data::Record(vec![("lo", self.lo.to_data()), ("hi", self.hi.to_data())])
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::hop::geom::Box {
            lo: AsData::from_data(d.field("lo")?)?,
            hi: AsData::from_data(d.field("hi")?)?,
        })
    }
}

impl AsData for crate::hop::model::Pad {
    fn to_data(&self) -> Data {
        Data::Record(
            vec![
                ("left", self.left.to_data()),
                ("right", self.right.to_data()),
                ("forward", self.forward.to_data()),
                ("back", self.back.to_data()),
                ("jump", self.jump.to_data()),
            ],
        )
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::hop::model::Pad {
            left: AsData::from_data(d.field("left")?)?,
            right: AsData::from_data(d.field("right")?)?,
            forward: AsData::from_data(d.field("forward")?)?,
            back: AsData::from_data(d.field("back")?)?,
            jump: AsData::from_data(d.field("jump")?)?,
        })
    }
}

impl AsData for crate::hop::model::Rgb {
    fn to_data(&self) -> Data {
        Data::Record(
            vec![
                ("r", self.r.to_data()),
                ("g", self.g.to_data()),
                ("b", self.b.to_data()),
            ],
        )
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::hop::model::Rgb {
            r: AsData::from_data(d.field("r")?)?,
            g: AsData::from_data(d.field("g")?)?,
            b: AsData::from_data(d.field("b")?)?,
        })
    }
}

impl AsData for crate::hop::model::Solid {
    fn to_data(&self) -> Data {
        Data::Record(
            vec![("box", self.r#box.to_data()), ("color", self.color.to_data())],
        )
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::hop::model::Solid {
            r#box: AsData::from_data(d.field("box")?)?,
            color: AsData::from_data(d.field("color")?)?,
        })
    }
}

impl AsData for crate::hop::model::Mover {
    fn to_data(&self) -> Data {
        Data::Record(
            vec![
                ("no", self.no.to_data()),
                ("home", self.home.to_data()),
                ("step", self.step.to_data()),
                ("span", self.span.to_data()),
                ("color", self.color.to_data()),
            ],
        )
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::hop::model::Mover {
            no: AsData::from_data(d.field("no")?)?,
            home: AsData::from_data(d.field("home")?)?,
            step: AsData::from_data(d.field("step")?)?,
            span: AsData::from_data(d.field("span")?)?,
            color: AsData::from_data(d.field("color")?)?,
        })
    }
}

impl AsData for crate::hop::model::Gem {
    fn to_data(&self) -> Data {
        Data::Record(vec![("no", self.no.to_data()), ("at", self.at.to_data())])
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::hop::model::Gem {
            no: AsData::from_data(d.field("no")?)?,
            at: AsData::from_data(d.field("at")?)?,
        })
    }
}

impl AsData for crate::hop::model::Crawler {
    fn to_data(&self) -> Data {
        Data::Record(
            vec![
                ("no", self.no.to_data()),
                ("at", self.at.to_data()),
                ("lo", self.lo.to_data()),
                ("hi", self.hi.to_data()),
                ("dir", self.dir.to_data()),
                ("speed", self.speed.to_data()),
            ],
        )
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::hop::model::Crawler {
            no: AsData::from_data(d.field("no")?)?,
            at: AsData::from_data(d.field("at")?)?,
            lo: AsData::from_data(d.field("lo")?)?,
            hi: AsData::from_data(d.field("hi")?)?,
            dir: AsData::from_data(d.field("dir")?)?,
            speed: AsData::from_data(d.field("speed")?)?,
        })
    }
}

impl AsData for crate::hop::model::Level {
    fn to_data(&self) -> Data {
        Data::Record(
            vec![
                ("name", self.name.to_data()),
                ("start", self.start.to_data()),
                ("solids", self.solids.to_data()),
                ("movers", self.movers.to_data()),
                ("gems", self.gems.to_data()),
                ("crawlers", self.crawlers.to_data()),
                ("goal", self.goal.to_data()),
            ],
        )
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::hop::model::Level {
            name: AsData::from_data(d.field("name")?)?,
            start: AsData::from_data(d.field("start")?)?,
            solids: AsData::from_data(d.field("solids")?)?,
            movers: AsData::from_data(d.field("movers")?)?,
            gems: AsData::from_data(d.field("gems")?)?,
            crawlers: AsData::from_data(d.field("crawlers")?)?,
            goal: AsData::from_data(d.field("goal")?)?,
        })
    }
}

impl AsData for crate::hop::model::Mode {
    fn to_data(&self) -> Data {
        match self {
            crate::hop::model::Mode::Playing => Data::Variant("Playing", Vec::new()),
            crate::hop::model::Mode::Won => Data::Variant("Won", Vec::new()),
            crate::hop::model::Mode::Lost => Data::Variant("Lost", Vec::new()),
        }
    }
    fn from_data(d: &Data) -> Option<Self> {
        let Data::Variant(name, _) = d else { return None };
        match *name {
            "Playing" => Some(crate::hop::model::Mode::Playing),
            "Won" => Some(crate::hop::model::Mode::Won),
            "Lost" => Some(crate::hop::model::Mode::Lost),
            _ => None,
        }
    }
}

impl AsData for crate::hop::model::Player {
    fn to_data(&self) -> Data {
        Data::Record(
            vec![
                ("at", self.at.to_data()),
                ("vel", self.vel.to_data()),
                ("grounded", self.grounded.to_data()),
                ("coyote", self.coyote.to_data()),
                ("jump_held", self.jump_held.to_data()),
                ("riding", self.riding.to_data()),
                ("yaw", self.yaw.to_data()),
            ],
        )
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::hop::model::Player {
            at: AsData::from_data(d.field("at")?)?,
            vel: AsData::from_data(d.field("vel")?)?,
            grounded: AsData::from_data(d.field("grounded")?)?,
            coyote: AsData::from_data(d.field("coyote")?)?,
            jump_held: AsData::from_data(d.field("jump_held")?)?,
            riding: AsData::from_data(d.field("riding")?)?,
            yaw: AsData::from_data(d.field("yaw")?)?,
        })
    }
}

impl AsData for crate::hop::physics::Axis {
    fn to_data(&self) -> Data {
        Data::Record(vec![("pos", self.pos.to_data()), ("vel", self.vel.to_data())])
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::hop::physics::Axis {
            pos: AsData::from_data(d.field("pos")?)?,
            vel: AsData::from_data(d.field("vel")?)?,
        })
    }
}

impl AsData for crate::hop::physics::Fall {
    fn to_data(&self) -> Data {
        Data::Record(
            vec![
                ("y", self.y.to_data()),
                ("vel", self.vel.to_data()),
                ("grounded", self.grounded.to_data()),
                ("riding", self.riding.to_data()),
            ],
        )
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::hop::physics::Fall {
            y: AsData::from_data(d.field("y")?)?,
            vel: AsData::from_data(d.field("vel")?)?,
            grounded: AsData::from_data(d.field("grounded")?)?,
            riding: AsData::from_data(d.field("riding")?)?,
        })
    }
}

impl AsData for crate::hop::physics::Placed {
    fn to_data(&self) -> Data {
        Data::Record(vec![("no", self.no.to_data()), ("box", self.r#box.to_data())])
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::hop::physics::Placed {
            no: AsData::from_data(d.field("no")?)?,
            r#box: AsData::from_data(d.field("box")?)?,
        })
    }
}

impl AsData for crate::hop::render::Asset {
    fn to_data(&self) -> Data {
        match self {
            crate::hop::render::Asset::PipMesh => Data::Variant("PipMesh", Vec::new()),
            crate::hop::render::Asset::CrawlerMesh => {
                Data::Variant("CrawlerMesh", Vec::new())
            }
            crate::hop::render::Asset::GemMesh => Data::Variant("GemMesh", Vec::new()),
            crate::hop::render::Asset::FlagMesh => Data::Variant("FlagMesh", Vec::new()),
        }
    }
    fn from_data(d: &Data) -> Option<Self> {
        let Data::Variant(name, _) = d else { return None };
        match *name {
            "PipMesh" => Some(crate::hop::render::Asset::PipMesh),
            "CrawlerMesh" => Some(crate::hop::render::Asset::CrawlerMesh),
            "GemMesh" => Some(crate::hop::render::Asset::GemMesh),
            "FlagMesh" => Some(crate::hop::render::Asset::FlagMesh),
            _ => None,
        }
    }
}

impl AsData for crate::hop::render::Draw {
    fn to_data(&self) -> Data {
        match self {
            crate::hop::render::Draw::Cube { r#box, color } => {
                Data::Variant(
                    "Cube",
                    vec![("box", r#box.to_data()), ("color", color.to_data())],
                )
            }
            crate::hop::render::Draw::Wires { r#box, color } => {
                Data::Variant(
                    "Wires",
                    vec![("box", r#box.to_data()), ("color", color.to_data())],
                )
            }
            crate::hop::render::Draw::Mesh { asset, at, yaw } => {
                Data::Variant(
                    "Mesh",
                    vec![
                        ("asset", asset.to_data()),
                        ("at", at.to_data()),
                        ("yaw", yaw.to_data()),
                    ],
                )
            }
        }
    }
    fn from_data(d: &Data) -> Option<Self> {
        let Data::Variant(name, _) = d else { return None };
        match *name {
            "Cube" => {
                Some(crate::hop::render::Draw::Cube {
                    r#box: AsData::from_data(d.field("box")?)?,
                    color: AsData::from_data(d.field("color")?)?,
                })
            }
            "Wires" => {
                Some(crate::hop::render::Draw::Wires {
                    r#box: AsData::from_data(d.field("box")?)?,
                    color: AsData::from_data(d.field("color")?)?,
                })
            }
            "Mesh" => {
                Some(crate::hop::render::Draw::Mesh {
                    asset: AsData::from_data(d.field("asset")?)?,
                    at: AsData::from_data(d.field("at")?)?,
                    yaw: AsData::from_data(d.field("yaw")?)?,
                })
            }
            _ => None,
        }
    }
}

impl AsData for crate::hop::render::Label {
    fn to_data(&self) -> Data {
        Data::Record(
            vec![
                ("x", self.x.to_data()),
                ("y", self.y.to_data()),
                ("size", self.size.to_data()),
                ("text", self.text.to_data()),
                ("color", self.color.to_data()),
            ],
        )
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::hop::render::Label {
            x: AsData::from_data(d.field("x")?)?,
            y: AsData::from_data(d.field("y")?)?,
            size: AsData::from_data(d.field("size")?)?,
            text: AsData::from_data(d.field("text")?)?,
            color: AsData::from_data(d.field("color")?)?,
        })
    }
}

impl AsData for crate::hop::render::Frame {
    fn to_data(&self) -> Data {
        Data::Record(
            vec![
                ("sky", self.sky.to_data()),
                ("eye", self.eye.to_data()),
                ("target", self.target.to_data()),
                ("time", self.time.to_data()),
                ("world", self.world.to_data()),
                ("labels", self.labels.to_data()),
            ],
        )
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::hop::render::Frame {
            sky: AsData::from_data(d.field("sky")?)?,
            eye: AsData::from_data(d.field("eye")?)?,
            target: AsData::from_data(d.field("target")?)?,
            time: AsData::from_data(d.field("time")?)?,
            world: AsData::from_data(d.field("world")?)?,
            labels: AsData::from_data(d.field("labels")?)?,
        })
    }
}

impl AsData for crate::hop::tests::runs::Hold {
    fn to_data(&self) -> Data {
        Data::Record(
            vec![("ticks", self.ticks.to_data()), ("keys", self.keys.to_data())],
        )
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::hop::tests::runs::Hold {
            ticks: AsData::from_data(d.field("ticks")?)?,
            keys: AsData::from_data(d.field("keys")?)?,
        })
    }
}

impl AsData for crate::hop::world::View {
    fn to_data(&self) -> Data {
        Data::Record(
            vec![
                ("stage", self.stage.to_data()),
                ("ticks", self.ticks.to_data()),
                ("player", self.player.to_data()),
                ("taken", self.taken.to_data()),
                ("crawlers", self.crawlers.to_data()),
                ("lives", self.lives.to_data()),
                ("mode", self.mode.to_data()),
            ],
        )
    }
    fn from_data(d: &Data) -> Option<Self> {
        if !matches!(d, Data::Record(_)) {
            return None;
        }
        Some(crate::hop::world::View {
            stage: AsData::from_data(d.field("stage")?)?,
            ticks: AsData::from_data(d.field("ticks")?)?,
            player: AsData::from_data(d.field("player")?)?,
            taken: AsData::from_data(d.field("taken")?)?,
            crawlers: AsData::from_data(d.field("crawlers")?)?,
            lives: AsData::from_data(d.field("lives")?)?,
            mode: AsData::from_data(d.field("mode")?)?,
        })
    }
}

/// The world as data: every state by name.
fn world_data(w: &World) -> Data {
    Data::Record(
        vec![
            ("stage", w.stage.to_data()),
            ("ticks", w.ticks.to_data()),
            ("player", w.player.to_data()),
            ("taken", w.taken.to_data()),
            ("crawlers", w.crawlers.to_data()),
            ("lives", w.lives.to_data()),
            ("mode", w.mode.to_data()),
        ],
    )
}

/// A world from data, when every state is of its Rust type (claims are checked apart).
fn data_world(d: &Data) -> Option<World> {
    Some(World {
        stage: AsData::from_data(d.field("stage")?)?,
        ticks: AsData::from_data(d.field("ticks")?)?,
        player: AsData::from_data(d.field("player")?)?,
        taken: AsData::from_data(d.field("taken")?)?,
        crawlers: AsData::from_data(d.field("crawlers")?)?,
        lives: AsData::from_data(d.field("lives")?)?,
        mode: AsData::from_data(d.field("mode")?)?,
    })
}

/// The app `apps.rust.SkyhopRust`: what its libraries' runtimes read.
pub static APP: App = App {
    name: "SkyhopRust",
    qualified: "apps.rust.SkyhopRust",
    module: "apps.rust",
    target: Some("rust"),
    shape: Some("game"),
    notes: &[
        Note {
            library: "game",
            name: "bin",
            args: &[
                NoteArg {
                    param: "name",
                    value: NoteValue::Text("skyhop"),
                },
            ],
        },
        Note {
            library: "game",
            name: "window",
            args: &[
                NoteArg {
                    param: "width",
                    value: NoteValue::Int(960),
                },
                NoteArg {
                    param: "height",
                    value: NoteValue::Int(540),
                },
                NoteArg {
                    param: "title",
                    value: NoteValue::Text("Skyhop"),
                },
            ],
        },
    ],
    params: &[
        AppParam {
            name: "start",
            decls: &[Decl::Rule(10)],
        },
        AppParam {
            name: "tick",
            decls: &[Decl::Rule(12)],
        },
        AppParam {
            name: "input",
            decls: &[Decl::Fn(0)],
        },
        AppParam {
            name: "frame",
            decls: &[Decl::Rule(9)],
        },
        AppParam {
            name: "dump",
            decls: &[Decl::Fn(1)],
        },
        AppParam {
            name: "report",
            decls: &[Decl::Fn(2)],
        },
    ],
    rules: &[
        Rule {
            name: "clear_background",
            qualified: "hop.screen.clear_background",
            params: &[
                Param {
                    name: "color",
                    ty: &TY_RGB,
                    has_default: false,
                },
            ],
            result: &TY_UNIT,
            triggers: &[],
            uses: &["screen.write"],
            writes: false,
            call: call_clear_background_0,
        },
        Rule {
            name: "begin_mode_3d",
            qualified: "hop.screen.begin_mode_3d",
            params: &[
                Param {
                    name: "eye",
                    ty: &TY_VEC3,
                    has_default: false,
                },
                Param {
                    name: "target",
                    ty: &TY_VEC3,
                    has_default: false,
                },
            ],
            result: &TY_UNIT,
            triggers: &[],
            uses: &["screen.write"],
            writes: false,
            call: call_begin_mode_3d_1,
        },
        Rule {
            name: "end_mode_3d",
            qualified: "hop.screen.end_mode_3d",
            params: &[],
            result: &TY_UNIT,
            triggers: &[],
            uses: &["screen.write"],
            writes: false,
            call: call_end_mode_3d_2,
        },
        Rule {
            name: "begin_lit",
            qualified: "hop.screen.begin_lit",
            params: &[
                Param {
                    name: "time",
                    ty: &TY_INT,
                    has_default: false,
                },
            ],
            result: &TY_UNIT,
            triggers: &[],
            uses: &["screen.write"],
            writes: false,
            call: call_begin_lit_3,
        },
        Rule {
            name: "end_lit",
            qualified: "hop.screen.end_lit",
            params: &[],
            result: &TY_UNIT,
            triggers: &[],
            uses: &["screen.write"],
            writes: false,
            call: call_end_lit_4,
        },
        Rule {
            name: "draw_cube",
            qualified: "hop.screen.draw_cube",
            params: &[
                Param {
                    name: "box",
                    ty: &TY_BOX,
                    has_default: false,
                },
                Param {
                    name: "color",
                    ty: &TY_RGB,
                    has_default: false,
                },
            ],
            result: &TY_UNIT,
            triggers: &[],
            uses: &["screen.write"],
            writes: false,
            call: call_draw_cube_5,
        },
        Rule {
            name: "draw_cube_wires",
            qualified: "hop.screen.draw_cube_wires",
            params: &[
                Param {
                    name: "box",
                    ty: &TY_BOX,
                    has_default: false,
                },
                Param {
                    name: "color",
                    ty: &TY_RGB,
                    has_default: false,
                },
            ],
            result: &TY_UNIT,
            triggers: &[],
            uses: &["screen.write"],
            writes: false,
            call: call_draw_cube_wires_6,
        },
        Rule {
            name: "draw_model",
            qualified: "hop.screen.draw_model",
            params: &[
                Param {
                    name: "asset",
                    ty: &TY_ASSET,
                    has_default: false,
                },
                Param {
                    name: "at",
                    ty: &TY_VEC3,
                    has_default: false,
                },
                Param {
                    name: "yaw",
                    ty: &TY_RANGE_0_359,
                    has_default: false,
                },
            ],
            result: &TY_UNIT,
            triggers: &[],
            uses: &["screen.write"],
            writes: false,
            call: call_draw_model_7,
        },
        Rule {
            name: "draw_text",
            qualified: "hop.screen.draw_text",
            params: &[
                Param {
                    name: "text",
                    ty: &TY_TEXT,
                    has_default: false,
                },
                Param {
                    name: "x",
                    ty: &TY_INT,
                    has_default: false,
                },
                Param {
                    name: "y",
                    ty: &TY_INT,
                    has_default: false,
                },
                Param {
                    name: "size",
                    ty: &TY_INT,
                    has_default: false,
                },
                Param {
                    name: "color",
                    ty: &TY_RGB,
                    has_default: false,
                },
            ],
            result: &TY_UNIT,
            triggers: &[],
            uses: &["screen.write"],
            writes: false,
            call: call_draw_text_8,
        },
        Rule {
            name: "present",
            qualified: "hop.screen.present",
            params: &[],
            result: &TY_UNIT,
            triggers: &[],
            uses: &[
                "screen.write",
                "world.stage.read",
                "world.ticks.read",
                "world.player.read",
                "world.taken.read",
                "world.crawlers.read",
                "world.lives.read",
                "world.mode.read",
            ],
            writes: false,
            call: call_present_9,
        },
        Rule {
            name: "start",
            qualified: "hop.world.start",
            params: &[
                Param {
                    name: "level",
                    ty: &TY_LEVEL_NO,
                    has_default: false,
                },
            ],
            result: &TY_UNIT,
            triggers: &[],
            uses: &[
                "stage.write",
                "ticks.write",
                "player.write",
                "taken.write",
                "crawlers.write",
                "lives.write",
                "mode.write",
                "out.write",
            ],
            writes: true,
            call: call_start_10,
        },
        Rule {
            name: "lose",
            qualified: "hop.world.lose",
            params: &[
                Param {
                    name: "why",
                    ty: &TY_TEXT,
                    has_default: false,
                },
            ],
            result: &TY_UNIT,
            triggers: &[],
            uses: &[],
            writes: false,
            call: call_lose_11,
        },
        Rule {
            name: "step",
            qualified: "hop.world.step",
            params: &[
                Param {
                    name: "pad",
                    ty: &TY_PAD,
                    has_default: false,
                },
            ],
            result: &TY_UNIT,
            triggers: &[
                Note {
                    library: "game",
                    name: "tick",
                    args: &[
                        NoteArg {
                            param: "rate",
                            value: NoteValue::Quantity(60, "Hz"),
                        },
                    ],
                },
            ],
            uses: &[
                "stage.read",
                "ticks.write",
                "player.write",
                "taken.write",
                "crawlers.write",
                "lives.write",
                "mode.write",
                "out.write",
            ],
            writes: true,
            call: call_step_12,
        },
    ],
    fns: &[
        Func {
            name: "read_pad",
            qualified: "hop.screen.read_pad",
            why: Some("arrows or WASD to run, space to jump"),
            params: &[],
            result: &TY_PAD,
            call: fn_read_pad_0,
        },
        Func {
            name: "dump",
            qualified: "hop.render.dump",
            why: None,
            params: &[],
            result: &TY_LIST_TEXT,
            call: fn_dump_1,
        },
        Func {
            name: "summary",
            qualified: "hop.world.summary",
            why: Some("one line that sums up a run, printed after the last tick"),
            params: &[],
            result: &TY_TEXT,
            call: fn_summary_2,
        },
        Func {
            name: "vec_text",
            qualified: "hop.geom.vec_text",
            why: None,
            params: &[
                Param {
                    name: "v",
                    ty: &TY_VEC3,
                    has_default: false,
                },
            ],
            result: &TY_TEXT,
            call: fn_vec_text_3,
        },
        Func {
            name: "body",
            qualified: "hop.physics.body",
            why: None,
            params: &[
                Param {
                    name: "at",
                    ty: &TY_VEC3,
                    has_default: false,
                },
            ],
            result: &TY_BOX,
            call: fn_body_4,
        },
        Func {
            name: "asset_name",
            qualified: "hop.render.asset_name",
            why: None,
            params: &[
                Param {
                    name: "a",
                    ty: &TY_ASSET,
                    has_default: false,
                },
            ],
            result: &TY_TEXT,
            call: fn_asset_name_5,
        },
        Func {
            name: "rgb_text",
            qualified: "hop.render.rgb_text",
            why: None,
            params: &[
                Param {
                    name: "c",
                    ty: &TY_RGB,
                    has_default: false,
                },
            ],
            result: &TY_TEXT,
            call: fn_rgb_text_6,
        },
    ],
    externs: &[
        Extern {
            library: "raylib",
            symbol: "ClearBackground",
            params: &[
                Param {
                    name: "color",
                    ty: &TY_RGB,
                    has_default: false,
                },
            ],
        },
        Extern {
            library: "raylib",
            symbol: "BeginMode3D",
            params: &[
                Param {
                    name: "eye",
                    ty: &TY_VEC3,
                    has_default: false,
                },
                Param {
                    name: "target",
                    ty: &TY_VEC3,
                    has_default: false,
                },
            ],
        },
        Extern {
            library: "raylib",
            symbol: "EndMode3D",
            params: &[],
        },
        Extern {
            library: "raylib",
            symbol: "BeginShaderMode",
            params: &[
                Param {
                    name: "time",
                    ty: &TY_INT,
                    has_default: false,
                },
            ],
        },
        Extern {
            library: "raylib",
            symbol: "EndShaderMode",
            params: &[],
        },
        Extern {
            library: "raylib",
            symbol: "DrawCube",
            params: &[
                Param {
                    name: "box",
                    ty: &TY_BOX,
                    has_default: false,
                },
                Param {
                    name: "color",
                    ty: &TY_RGB,
                    has_default: false,
                },
            ],
        },
        Extern {
            library: "raylib",
            symbol: "DrawCubeWires",
            params: &[
                Param {
                    name: "box",
                    ty: &TY_BOX,
                    has_default: false,
                },
                Param {
                    name: "color",
                    ty: &TY_RGB,
                    has_default: false,
                },
            ],
        },
        Extern {
            library: "raylib",
            symbol: "DrawModelEx",
            params: &[
                Param {
                    name: "asset",
                    ty: &TY_ASSET,
                    has_default: false,
                },
                Param {
                    name: "at",
                    ty: &TY_VEC3,
                    has_default: false,
                },
                Param {
                    name: "yaw",
                    ty: &TY_RANGE_0_359,
                    has_default: false,
                },
            ],
        },
        Extern {
            library: "raylib",
            symbol: "DrawText",
            params: &[
                Param {
                    name: "text",
                    ty: &TY_TEXT,
                    has_default: false,
                },
                Param {
                    name: "x",
                    ty: &TY_INT,
                    has_default: false,
                },
                Param {
                    name: "y",
                    ty: &TY_INT,
                    has_default: false,
                },
                Param {
                    name: "size",
                    ty: &TY_INT,
                    has_default: false,
                },
                Param {
                    name: "color",
                    ty: &TY_RGB,
                    has_default: false,
                },
            ],
        },
        Extern {
            library: "raylib",
            symbol: "IsKeyDown",
            params: &[
                Param {
                    name: "key",
                    ty: &TY_INT,
                    has_default: false,
                },
            ],
        },
        Extern {
            library: "raylib",
            symbol: "CheckCollisionBoxes",
            params: &[
                Param {
                    name: "a",
                    ty: &TY_BOX,
                    has_default: false,
                },
                Param {
                    name: "b",
                    ty: &TY_BOX,
                    has_default: false,
                },
            ],
        },
    ],
    states: &[
        State {
            name: "stage",
            qualified: "hop.world.stage",
            ty: &TY_LEVEL,
        },
        State {
            name: "ticks",
            qualified: "hop.world.ticks",
            ty: &TY_INT,
        },
        State {
            name: "player",
            qualified: "hop.world.player",
            ty: &TY_PLAYER,
        },
        State {
            name: "taken",
            qualified: "hop.world.taken",
            ty: &TY_SET_INT,
        },
        State {
            name: "crawlers",
            qualified: "hop.world.crawlers",
            ty: &TY_MAP_INT_CRAWLER,
        },
        State {
            name: "lives",
            qualified: "hop.world.lives",
            ty: &TY_RANGE_0_3,
        },
        State {
            name: "mode",
            qualified: "hop.world.mode",
            ty: &TY_MODE,
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

/// `hop.model.Rgb`.
pub static TY_RGB: Ty = Ty {
    name: "Rgb",
    shape: Shape::Record(
        &[("r", &TY_RANGE_0_255), ("g", &TY_RANGE_0_255), ("b", &TY_RANGE_0_255)],
    ),
    range: None,
    admits: |d| rt::data::admits::<
        crate::hop::model::Rgb,
    >(d, |v| crate::conform::rgb(v).unwrap_or(false)),
};

/// `()`.
pub static TY_UNIT: Ty = Ty {
    name: "()",
    shape: Shape::Unit,
    range: None,
    admits: |d| rt::data::admits::<()>(d, |_| true),
};

/// `hop.geom.Vec3`.
pub static TY_VEC3: Ty = Ty {
    name: "Vec3",
    shape: Shape::Record(&[("x", &TY_SUB), ("y", &TY_SUB), ("z", &TY_SUB)]),
    range: None,
    admits: |d| rt::data::admits::<
        crate::hop::geom::Vec3,
    >(d, |v| crate::conform::vec3(v).unwrap_or(false)),
};

/// `Int`.
pub static TY_INT: Ty = Ty {
    name: "Int",
    shape: Shape::Int,
    range: None,
    admits: |d| rt::data::admits::<Int>(d, |_| true),
};

/// `hop.geom.Box`.
pub static TY_BOX: Ty = Ty {
    name: "Box",
    shape: Shape::Record(&[("lo", &TY_VEC3), ("hi", &TY_VEC3)]),
    range: None,
    admits: |d| rt::data::admits::<
        crate::hop::geom::Box,
    >(d, |v| crate::conform::r#box(v).unwrap_or(false)),
};

/// `hop.render.Asset`.
pub static TY_ASSET: Ty = Ty {
    name: "Asset",
    shape: Shape::Sum(
        &[("PipMesh", &[]), ("CrawlerMesh", &[]), ("GemMesh", &[]), ("FlagMesh", &[])],
    ),
    range: None,
    admits: |d| rt::data::admits::<crate::hop::render::Asset>(d, |_| true),
};

/// `0..=359`.
pub static TY_RANGE_0_359: Ty = Ty {
    name: "0..=359",
    shape: Shape::Int,
    range: Some((0, 359)),
    admits: |d| rt::data::admits::<
        Int,
    >(d, |v| crate::conform::range_0_359(v).unwrap_or(false)),
};

/// `Text`.
pub static TY_TEXT: Ty = Ty {
    name: "Text",
    shape: Shape::Text,
    range: None,
    admits: |d| rt::data::admits::<String>(d, |_| true),
};

/// `hop.levels.LevelNo`.
pub static TY_LEVEL_NO: Ty = Ty {
    name: "LevelNo",
    shape: Shape::Int,
    range: Some((1, 2)),
    admits: |d| rt::data::admits::<
        Int,
    >(d, |v| crate::conform::level_no(v).unwrap_or(false)),
};

/// `hop.model.Pad`.
pub static TY_PAD: Ty = Ty {
    name: "Pad",
    shape: Shape::Record(
        &[
            ("left", &TY_BOOL),
            ("right", &TY_BOOL),
            ("forward", &TY_BOOL),
            ("back", &TY_BOOL),
            ("jump", &TY_BOOL),
        ],
    ),
    range: None,
    admits: |d| rt::data::admits::<crate::hop::model::Pad>(d, |_| true),
};

/// `List[Text]`.
pub static TY_LIST_TEXT: Ty = Ty {
    name: "List[Text]",
    shape: Shape::List(&TY_TEXT),
    range: None,
    admits: |d| rt::data::admits::<Vec<String>>(d, |_| true),
};

/// `hop.model.Level`.
pub static TY_LEVEL: Ty = Ty {
    name: "Level",
    shape: Shape::Record(
        &[
            ("name", &TY_TEXT),
            ("start", &TY_VEC3),
            ("solids", &TY_LIST_SOLID),
            ("movers", &TY_LIST_MOVER),
            ("gems", &TY_LIST_GEM),
            ("crawlers", &TY_LIST_CRAWLER),
            ("goal", &TY_VEC3),
        ],
    ),
    range: None,
    admits: |d| rt::data::admits::<
        crate::hop::model::Level,
    >(d, |v| crate::conform::level(v).unwrap_or(false)),
};

/// `hop.model.Player`.
pub static TY_PLAYER: Ty = Ty {
    name: "Player",
    shape: Shape::Record(
        &[
            ("at", &TY_VEC3),
            ("vel", &TY_VEC3),
            ("grounded", &TY_BOOL),
            ("coyote", &TY_RANGE_0_6),
            ("jump_held", &TY_BOOL),
            ("riding", &TY_OPTION_INT),
            ("yaw", &TY_RANGE_0_359),
        ],
    ),
    range: None,
    admits: |d| rt::data::admits::<
        crate::hop::model::Player,
    >(d, |v| crate::conform::player(v).unwrap_or(false)),
};

/// `Set[Int]`.
pub static TY_SET_INT: Ty = Ty {
    name: "Set[Int]",
    shape: Shape::Set(&TY_INT),
    range: None,
    admits: |d| rt::data::admits::<BTreeSet<Int>>(d, |_| true),
};

/// `Map[Int, hop.model.Crawler]`.
pub static TY_MAP_INT_CRAWLER: Ty = Ty {
    name: "Map[Int, hop.model.Crawler]",
    shape: Shape::Map(&TY_INT, &TY_CRAWLER),
    range: None,
    admits: |d| rt::data::admits::<
        BTreeMap<Int, crate::hop::model::Crawler>,
    >(d, |v| crate::conform::map_int_crawler(v).unwrap_or(false)),
};

/// `0..=3`.
pub static TY_RANGE_0_3: Ty = Ty {
    name: "0..=3",
    shape: Shape::Int,
    range: Some((0, 3)),
    admits: |d| rt::data::admits::<
        Int,
    >(d, |v| crate::conform::range_0_3(v).unwrap_or(false)),
};

/// `hop.model.Mode`.
pub static TY_MODE: Ty = Ty {
    name: "Mode",
    shape: Shape::Sum(&[("Playing", &[]), ("Won", &[]), ("Lost", &[])]),
    range: None,
    admits: |d| rt::data::admits::<crate::hop::model::Mode>(d, |_| true),
};

/// `0..=255`.
pub static TY_RANGE_0_255: Ty = Ty {
    name: "0..=255",
    shape: Shape::Int,
    range: Some((0, 255)),
    admits: |d| rt::data::admits::<
        Int,
    >(d, |v| crate::conform::range_0_255(v).unwrap_or(false)),
};

/// `hop.geom.Sub`.
pub static TY_SUB: Ty = Ty {
    name: "Sub",
    shape: Shape::Int,
    range: Some((-1048576, 1048576)),
    admits: |d| rt::data::admits::<Int>(d, |v| crate::conform::sub(v).unwrap_or(false)),
};

/// `Bool`.
pub static TY_BOOL: Ty = Ty {
    name: "Bool",
    shape: Shape::Bool,
    range: None,
    admits: |d| rt::data::admits::<bool>(d, |_| true),
};

/// `List[hop.model.Solid]`.
pub static TY_LIST_SOLID: Ty = Ty {
    name: "List[hop.model.Solid]",
    shape: Shape::List(&TY_SOLID),
    range: None,
    admits: |d| rt::data::admits::<
        Vec<crate::hop::model::Solid>,
    >(d, |v| crate::conform::list_solid(v).unwrap_or(false)),
};

/// `List[hop.model.Mover]`.
pub static TY_LIST_MOVER: Ty = Ty {
    name: "List[hop.model.Mover]",
    shape: Shape::List(&TY_MOVER),
    range: None,
    admits: |d| rt::data::admits::<
        Vec<crate::hop::model::Mover>,
    >(d, |v| crate::conform::list_mover(v).unwrap_or(false)),
};

/// `List[hop.model.Gem]`.
pub static TY_LIST_GEM: Ty = Ty {
    name: "List[hop.model.Gem]",
    shape: Shape::List(&TY_GEM),
    range: None,
    admits: |d| rt::data::admits::<
        Vec<crate::hop::model::Gem>,
    >(d, |v| crate::conform::list_gem(v).unwrap_or(false)),
};

/// `List[hop.model.Crawler]`.
pub static TY_LIST_CRAWLER: Ty = Ty {
    name: "List[hop.model.Crawler]",
    shape: Shape::List(&TY_CRAWLER),
    range: None,
    admits: |d| rt::data::admits::<
        Vec<crate::hop::model::Crawler>,
    >(d, |v| crate::conform::list_crawler(v).unwrap_or(false)),
};

/// `0..=6`.
pub static TY_RANGE_0_6: Ty = Ty {
    name: "0..=6",
    shape: Shape::Int,
    range: Some((0, 6)),
    admits: |d| rt::data::admits::<
        Int,
    >(d, |v| crate::conform::range_0_6(v).unwrap_or(false)),
};

/// `Option[Int]`.
pub static TY_OPTION_INT: Ty = Ty {
    name: "Option[Int]",
    shape: Shape::Option(&TY_INT),
    range: None,
    admits: |d| rt::data::admits::<Opt<Int>>(d, |_| true),
};

/// `hop.model.Crawler`.
pub static TY_CRAWLER: Ty = Ty {
    name: "Crawler",
    shape: Shape::Record(
        &[
            ("no", &TY_INT),
            ("at", &TY_VEC3),
            ("lo", &TY_INT),
            ("hi", &TY_INT),
            ("dir", &TY_RANGE_M1_1),
            ("speed", &TY_RANGE_1_8),
        ],
    ),
    range: None,
    admits: |d| rt::data::admits::<
        crate::hop::model::Crawler,
    >(d, |v| crate::conform::crawler(v).unwrap_or(false)),
};

/// `hop.model.Solid`.
pub static TY_SOLID: Ty = Ty {
    name: "Solid",
    shape: Shape::Record(&[("box", &TY_BOX), ("color", &TY_RGB)]),
    range: None,
    admits: |d| rt::data::admits::<
        crate::hop::model::Solid,
    >(d, |v| crate::conform::solid(v).unwrap_or(false)),
};

/// `hop.model.Mover`.
pub static TY_MOVER: Ty = Ty {
    name: "Mover",
    shape: Shape::Record(
        &[
            ("no", &TY_INT),
            ("home", &TY_BOX),
            ("step", &TY_VEC3),
            ("span", &TY_RANGE_1_1024),
            ("color", &TY_RGB),
        ],
    ),
    range: None,
    admits: |d| rt::data::admits::<
        crate::hop::model::Mover,
    >(d, |v| crate::conform::mover(v).unwrap_or(false)),
};

/// `hop.model.Gem`.
pub static TY_GEM: Ty = Ty {
    name: "Gem",
    shape: Shape::Record(&[("no", &TY_INT), ("at", &TY_VEC3)]),
    range: None,
    admits: |d| rt::data::admits::<
        crate::hop::model::Gem,
    >(d, |v| crate::conform::gem(v).unwrap_or(false)),
};

/// `-1..=1`.
pub static TY_RANGE_M1_1: Ty = Ty {
    name: "-1..=1",
    shape: Shape::Int,
    range: Some((-1, 1)),
    admits: |d| rt::data::admits::<
        Int,
    >(d, |v| crate::conform::range_minus1_1(v).unwrap_or(false)),
};

/// `1..=8`.
pub static TY_RANGE_1_8: Ty = Ty {
    name: "1..=8",
    shape: Shape::Int,
    range: Some((1, 8)),
    admits: |d| rt::data::admits::<
        Int,
    >(d, |v| crate::conform::range_1_8(v).unwrap_or(false)),
};

/// `1..=1024`.
pub static TY_RANGE_1_1024: Ty = Ty {
    name: "1..=1024",
    shape: Shape::Int,
    range: Some((1, 1024)),
    admits: |d| rt::data::admits::<
        Int,
    >(d, |v| crate::conform::range_1_1024(v).unwrap_or(false)),
};
