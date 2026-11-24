//! Module `hop.model`, printed by spec-lang from `src/hop/model.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.

use crate::rt::{self, Int, Opt, R, Show};

/// one frame of controls, sampled once per tick
///
/// Record `hop.model.Pad`, `src/hop/model.slg:6`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pad {
    pub left: bool,
    pub right: bool,
    pub forward: bool,
    pub back: bool,
    pub jump: bool,
}

impl Show for Pad {
    fn show(&self) -> String {
        rt::show::record(
            "Pad",
            &[
                ("left", self.left.show()),
                ("right", self.right.show()),
                ("forward", self.forward.show()),
                ("back", self.back.show()),
                ("jump", self.jump.show()),
            ],
        )
    }
}

impl Pad {
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new(
        left: Option<bool>,
        right: Option<bool>,
        forward: Option<bool>,
        back: Option<bool>,
        jump: Option<bool>,
    ) -> R<Self> {
        let left = left.unwrap_or_default();
        let right = right.unwrap_or_default();
        let forward = forward.unwrap_or_default();
        let back = back.unwrap_or_default();
        let jump = jump.unwrap_or_default();
        Ok(Pad {
            left,
            right,
            forward,
            back,
            jump,
        })
    }
}

/// Record `hop.model.Rgb`, `src/hop/model.slg:9`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rgb {
    pub r: Int,
    pub g: Int,
    pub b: Int,
}

impl Show for Rgb {
    fn show(&self) -> String {
        rt::show::record(
            "Rgb",
            &[("r", self.r.show()), ("g", self.g.show()), ("b", self.b.show())],
        )
    }
}

impl Rgb {
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new(r: Int, g: Int, b: Int) -> R<Self> {
        rt::claim(crate::conform::range_0_255(&r)?, "field `r`")?;
        rt::claim(crate::conform::range_0_255(&g)?, "field `g`")?;
        rt::claim(crate::conform::range_0_255(&b)?, "field `b`")?;
        Ok(Rgb { r, g, b })
    }
}

/// Record `hop.model.Solid`, `src/hop/model.slg:11`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Solid {
    pub r#box: crate::hop::geom::Box,
    pub color: Rgb,
}

impl Show for Solid {
    fn show(&self) -> String {
        rt::show::record(
            "Solid",
            &[("box", self.r#box.show()), ("color", self.color.show())],
        )
    }
}

impl Solid {
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new(r#box: crate::hop::geom::Box, color: Rgb) -> R<Self> {
        rt::claim(crate::conform::r#box(&r#box)?, "field `box`")?;
        rt::claim(crate::conform::rgb(&color)?, "field `color`")?;
        Ok(Solid { r#box, color })
    }
}

/// a one-way platform: it carries whoever lands on it from above and never blocks from the side or below
///
/// Record `hop.model.Mover`, `src/hop/model.slg:13`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Mover {
    pub no: Int,
    pub home: crate::hop::geom::Box,
    pub step: crate::hop::geom::Vec3,
    pub span: Int,
    pub color: Rgb,
}

impl Show for Mover {
    fn show(&self) -> String {
        rt::show::record(
            "Mover",
            &[
                ("no", self.no.show()),
                ("home", self.home.show()),
                ("step", self.step.show()),
                ("span", self.span.show()),
                ("color", self.color.show()),
            ],
        )
    }
}

impl Mover {
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new(
        no: Int,
        home: crate::hop::geom::Box,
        step: crate::hop::geom::Vec3,
        span: Int,
        color: Rgb,
    ) -> R<Self> {
        rt::claim(crate::conform::r#box(&home)?, "field `home`")?;
        rt::claim(crate::conform::vec3(&step)?, "field `step`")?;
        rt::claim(crate::conform::range_1_1024(&span)?, "field `span`")?;
        rt::claim(crate::conform::rgb(&color)?, "field `color`")?;
        Ok(Mover {
            no,
            home,
            step,
            span,
            color,
        })
    }
}

/// Record `hop.model.Gem`, `src/hop/model.slg:16`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Gem {
    pub no: Int,
    pub at: crate::hop::geom::Vec3,
}

impl Show for Gem {
    fn show(&self) -> String {
        rt::show::record("Gem", &[("no", self.no.show()), ("at", self.at.show())])
    }
}

impl Gem {
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new(no: Int, at: crate::hop::geom::Vec3) -> R<Self> {
        rt::claim(crate::conform::vec3(&at)?, "field `at`")?;
        Ok(Gem { no, at })
    }
}

/// a crawler patrols along x between lo and hi
///
/// Record `hop.model.Crawler`, `src/hop/model.slg:18`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Crawler {
    pub no: Int,
    pub at: crate::hop::geom::Vec3,
    pub lo: Int,
    pub hi: Int,
    pub dir: Int,
    pub speed: Int,
}

impl Show for Crawler {
    fn show(&self) -> String {
        rt::show::record(
            "Crawler",
            &[
                ("no", self.no.show()),
                ("at", self.at.show()),
                ("lo", self.lo.show()),
                ("hi", self.hi.show()),
                ("dir", self.dir.show()),
                ("speed", self.speed.show()),
            ],
        )
    }
}

impl Crawler {
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new(
        no: Int,
        at: crate::hop::geom::Vec3,
        lo: Int,
        hi: Int,
        dir: Int,
        speed: Int,
    ) -> R<Self> {
        rt::claim(crate::conform::vec3(&at)?, "field `at`")?;
        rt::claim(crate::conform::range_minus1_1(&dir)?, "field `dir`")?;
        rt::claim(crate::conform::range_1_8(&speed)?, "field `speed`")?;
        Ok(Crawler {
            no,
            at,
            lo,
            hi,
            dir,
            speed,
        })
    }
}

/// Record `hop.model.Level`, `src/hop/model.slg:21`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Level {
    pub name: String,
    pub start: crate::hop::geom::Vec3,
    pub solids: Vec<Solid>,
    pub movers: Vec<Mover>,
    pub gems: Vec<Gem>,
    pub crawlers: Vec<Crawler>,
    pub goal: crate::hop::geom::Vec3,
}

impl Show for Level {
    fn show(&self) -> String {
        rt::show::record(
            "Level",
            &[
                ("name", self.name.show()),
                ("start", self.start.show()),
                ("solids", self.solids.show()),
                ("movers", self.movers.show()),
                ("gems", self.gems.show()),
                ("crawlers", self.crawlers.show()),
                ("goal", self.goal.show()),
            ],
        )
    }
}

impl Level {
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new(
        name: String,
        start: crate::hop::geom::Vec3,
        solids: Vec<Solid>,
        movers: Option<Vec<Mover>>,
        gems: Option<Vec<Gem>>,
        crawlers: Option<Vec<Crawler>>,
        goal: crate::hop::geom::Vec3,
    ) -> R<Self> {
        rt::claim(crate::conform::vec3(&start)?, "field `start`")?;
        rt::claim(crate::conform::list_solid(&solids)?, "field `solids`")?;
        let movers = movers.unwrap_or_default();
        rt::claim(crate::conform::list_mover(&movers)?, "field `movers`")?;
        let gems = gems.unwrap_or_default();
        rt::claim(crate::conform::list_gem(&gems)?, "field `gems`")?;
        let crawlers = crawlers.unwrap_or_default();
        rt::claim(crate::conform::list_crawler(&crawlers)?, "field `crawlers`")?;
        rt::claim(crate::conform::vec3(&goal)?, "field `goal`")?;
        Ok(Level {
            name,
            start,
            solids,
            movers,
            gems,
            crawlers,
            goal,
        })
    }
}

/// Sum `hop.model.Mode`, `src/hop/model.slg:31`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mode {
    Playing,
    Won,
    Lost,
}

impl Show for Mode {
    fn show(&self) -> String {
        match self {
            Mode::Playing => "Playing".to_string(),
            Mode::Won => "Won".to_string(),
            Mode::Lost => "Lost".to_string(),
        }
    }
}

/// riding names the mover the feet are on, so the body moves with it
///
/// Record `hop.model.Player`, `src/hop/model.slg:33`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Player {
    pub at: crate::hop::geom::Vec3,
    pub vel: crate::hop::geom::Vec3,
    pub grounded: bool,
    pub coyote: Int,
    pub jump_held: bool,
    pub riding: Opt<Int>,
    pub yaw: Int,
}

impl Show for Player {
    fn show(&self) -> String {
        rt::show::record(
            "Player",
            &[
                ("at", self.at.show()),
                ("vel", self.vel.show()),
                ("grounded", self.grounded.show()),
                ("coyote", self.coyote.show()),
                ("jump_held", self.jump_held.show()),
                ("riding", self.riding.show()),
                ("yaw", self.yaw.show()),
            ],
        )
    }
}

impl Player {
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new(
        at: crate::hop::geom::Vec3,
        vel: Option<crate::hop::geom::Vec3>,
        grounded: Option<bool>,
        coyote: Option<Int>,
        jump_held: Option<bool>,
        riding: Option<Opt<Int>>,
        yaw: Option<Int>,
    ) -> R<Self> {
        rt::claim(crate::conform::vec3(&at)?, "field `at`")?;
        let vel = match vel {
            Some(v) => v,
            None => crate::hop::geom::Vec3::new(0, 0, 0)?,
        };
        rt::claim(crate::conform::vec3(&vel)?, "field `vel`")?;
        let grounded = grounded.unwrap_or_default();
        let coyote = coyote.unwrap_or_default();
        rt::claim(crate::conform::range_0_6(&coyote)?, "field `coyote`")?;
        let jump_held = jump_held.unwrap_or_default();
        let riding = riding.unwrap_or_default();
        let yaw = yaw.unwrap_or_default();
        rt::claim(crate::conform::range_0_359(&yaw)?, "field `yaw`")?;
        Ok(Player {
            at,
            vel,
            grounded,
            coyote,
            jump_held,
            riding,
            yaw,
        })
    }
}
