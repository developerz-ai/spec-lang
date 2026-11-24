// Projected by hand from Pad, Rgb, Solid, Mover, Gem, Crawler, Level, Mode, Player (src/hop/model.slg). Stand-in for spec-lang build output; change the spec, not this file.
//! What a level and the bodies in it are. Data only.

use std::fmt;

use crate::geom::{Box, Vec3};

/// One frame of controls, sampled once per tick. Every field defaults to `false`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Pad {
    pub left: bool,
    pub right: bool,
    pub forward: bool,
    pub back: bool,
    pub jump: bool,
}

/// `type Rgb(r: 0..=255, g: 0..=255, b: 0..=255)`: each range is exactly a `u8`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

/// `rgb_text` (hop.render): `"{r},{g},{b}"`.
impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},{}", self.r, self.g, self.b)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Solid {
    pub r#box: Box,
    pub color: Rgb,
}

/// A one-way platform. `span: 1..=1024`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mover {
    pub no: i32,
    pub home: Box,
    pub step: Vec3,
    pub span: i32,
    pub color: Rgb,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gem {
    pub no: i32,
    pub at: Vec3,
}

/// A crawler patrols along x between `lo` and `hi`. `dir: -1..=1`, `speed: 1..=8`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Crawler {
    pub no: i32,
    pub at: Vec3,
    pub lo: i32,
    pub hi: i32,
    pub dir: i32,
    pub speed: i32,
}

/// A level. Its lists are `&'static` slices: both levels are `const` in the spec, so they live in read-only data and a `start` copies no list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Level {
    pub name: &'static str,
    pub start: Vec3,
    pub solids: &'static [Solid],
    pub movers: &'static [Mover],
    pub gems: &'static [Gem],
    pub crawlers: &'static [Crawler],
    pub goal: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Playing,
    Won,
    Lost,
}

/// The hero. `coyote: 0..=6`, `yaw: 0..=359`; `riding` names the mover the feet are on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Player {
    pub at: Vec3,
    pub vel: Vec3,
    pub grounded: bool,
    pub coyote: i32,
    pub jump_held: bool,
    pub riding: Option<i32>,
    pub yaw: i32,
}

impl Player {
    /// `Player(at)` with every other field at its declared default.
    pub const fn new(at: Vec3) -> Self {
        Self {
            at,
            vel: Vec3::new(0, 0, 0),
            grounded: false,
            coyote: 0,
            jump_held: false,
            riding: None,
            yaw: 0,
        }
    }
}
