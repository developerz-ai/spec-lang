// Projected by hand from Asset, Draw, Label, Frame, sky, edge, ink, eye_offset, look_height, asset_name, bob, block, banner, scene, rgb_text, draw_text, frame_lines, dump (src/hop/render.slg). Stand-in for spec-lang build output; change the spec, not this file.
//! What one frame shows, as data. Pure; the screen module draws it.

use std::fmt;

use crate::geom::{self, Box, Vec3};
use crate::model::{Mode, Rgb};
use crate::physics;
use crate::world::{View, World};

/// Each mesh is one model file the game loads at start. The declared order is the load order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Asset {
    PipMesh,
    CrawlerMesh,
    GemMesh,
    FlagMesh,
}

impl Asset {
    pub const ALL: [Self; 4] = [
        Self::PipMesh,
        Self::CrawlerMesh,
        Self::GemMesh,
        Self::FlagMesh,
    ];

    pub const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Draw {
    Cube { r#box: Box, color: Rgb },
    Wires { r#box: Box, color: Rgb },
    Mesh { asset: Asset, at: Vec3, yaw: i32 },
}

/// The HUD's texts are fixed words around small integers: a `String` each, built once per frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    pub x: i32,
    pub y: i32,
    pub size: i32,
    pub text: String,
    pub color: Rgb,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub sky: Rgb,
    pub eye: Vec3,
    pub target: Vec3,
    pub time: i32,
    pub world: Vec<Draw>,
    pub labels: Vec<Label>,
}

const SKY: Rgb = Rgb::new(135, 206, 235);
const EDGE: Rgb = Rgb::new(30, 40, 30);
const INK: Rgb = Rgb::new(255, 255, 255);

/// The camera looks at the hero from the side and a little above, at a fixed distance.
pub const EYE_OFFSET: Vec3 = Vec3::new(0, 512, 2048);
pub const LOOK_HEIGHT: i32 = 192;

pub const fn asset_name(a: Asset) -> &'static str {
    match a {
        Asset::PipMesh => "pip",
        Asset::CrawlerMesh => "crawler",
        Asset::GemMesh => "gem",
        Asset::FlagMesh => "flag",
    }
}

/// A gem bobs a quarter of a metre and turns three degrees a tick.
pub const fn bob(t: i32) -> i32 {
    geom::triangle(t, 32) * 2 - 32
}

fn block(draws: &mut Vec<Draw>, r#box: Box, color: Rgb) {
    draws.push(Draw::Cube { r#box, color });
    draws.push(Draw::Wires { r#box, color: EDGE });
}

fn banner(m: Mode) -> Option<Label> {
    match m {
        Mode::Playing => None,
        Mode::Won => Some(Label {
            x: 380,
            y: 250,
            size: 40,
            text: "level clear".to_owned(),
            color: INK,
        }),
        Mode::Lost => Some(Label {
            x: 400,
            y: 250,
            size: 40,
            text: "game over".to_owned(),
            color: INK,
        }),
    }
}

/// Solids, then movers, gems, crawlers, the flag and last the hero, so the order of the frame is fixed.
/// The draw list is allocated once at its final size: every count is read from the view first.
pub fn scene(v: View<'_>) -> Frame {
    let target = geom::plus(v.player.at, Vec3::new(0, LOOK_HEIGHT, 0));
    let stage = v.stage;
    let mut draws = Vec::with_capacity(
        2 * (stage.solids.len() + stage.movers.len()) + stage.gems.len() + v.crawlers.len() + 2,
    );
    for s in stage.solids {
        block(&mut draws, s.r#box, s.color);
    }
    for m in stage.movers {
        block(&mut draws, physics::mover_box(m, v.ticks), m.color);
    }
    let spin = (v.ticks * 3).rem_euclid(360);
    let lift = bob(v.ticks);
    for g in stage.gems.iter().filter(|g| !v.taken.contains(&g.no)) {
        draws.push(Draw::Mesh {
            asset: Asset::GemMesh,
            at: g.at.with_y(g.at.y + lift),
            yaw: spin,
        });
    }
    for c in v.crawlers.values() {
        draws.push(Draw::Mesh {
            asset: Asset::CrawlerMesh,
            at: c.at,
            yaw: if c.dir < 0 { 180 } else { 0 },
        });
    }
    draws.push(Draw::Mesh {
        asset: Asset::FlagMesh,
        at: stage.goal,
        yaw: 0,
    });
    draws.push(Draw::Mesh {
        asset: Asset::PipMesh,
        at: v.player.at,
        yaw: v.player.yaw,
    });
    let mut labels = vec![
        Label {
            x: 16,
            y: 12,
            size: 20,
            text: format!("gems {}/{}", v.taken.len(), stage.gems.len()),
            color: INK,
        },
        Label {
            x: 16,
            y: 38,
            size: 20,
            text: format!("lives {}", v.lives),
            color: INK,
        },
    ];
    labels.extend(banner(v.mode));
    Frame {
        sky: SKY,
        eye: geom::plus(target, EYE_OFFSET),
        target,
        time: v.ticks,
        world: draws,
        labels,
    }
}

pub fn rgb_text(c: Rgb) -> String {
    c.to_string()
}

/// `draw_text`, written straight into the formatter.
impl fmt::Display for Draw {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Draw::Cube { r#box, color } => write!(f, "cube {} {} {color}", r#box.lo, r#box.hi),
            Draw::Wires { r#box, color } => write!(f, "wires {} {} {color}", r#box.lo, r#box.hi),
            Draw::Mesh { asset, at, yaw } => write!(f, "mesh {} {at} {yaw}", asset_name(*asset)),
        }
    }
}

pub fn draw_text(d: &Draw) -> String {
    d.to_string()
}

/// A frame as text, one line per call the screen will make.
pub fn frame_lines(f: &Frame) -> Vec<String> {
    let mut lines = Vec::with_capacity(3 + f.world.len() + f.labels.len());
    lines.push(format!("sky {}", f.sky));
    lines.push(format!("camera {} {}", f.eye, f.target));
    lines.push(format!("time {}", f.time));
    lines.extend(f.world.iter().map(draw_text));
    lines.extend(
        f.labels
            .iter()
            .map(|l| format!("label {} {} {} {} {}", l.x, l.y, l.size, l.color, l.text)),
    );
    lines
}

/// `frame_lines` each followed by `\n`, into one buffer: the shape `draw` prints.
pub fn write_frame(f: &Frame, into: &mut String) {
    for line in frame_lines(f) {
        into.push_str(&line);
        into.push('\n');
    }
}

pub fn dump(world: &World) -> Vec<String> {
    frame_lines(&scene(world.view()))
}
