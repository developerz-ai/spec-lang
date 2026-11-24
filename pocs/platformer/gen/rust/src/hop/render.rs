//! Module `hop.render`, printed by spec-lang from `src/hop/render.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.

use crate::rt::{self, Int, R, Show};
use crate::machine::Machine;

/// each mesh is one model file the game loads at start
///
/// Sum `hop.render.Asset`, `src/hop/render.slg:12`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Asset {
    PipMesh,
    CrawlerMesh,
    GemMesh,
    FlagMesh,
}

impl Show for Asset {
    fn show(&self) -> String {
        match self {
            Asset::PipMesh => "PipMesh".to_string(),
            Asset::CrawlerMesh => "CrawlerMesh".to_string(),
            Asset::GemMesh => "GemMesh".to_string(),
            Asset::FlagMesh => "FlagMesh".to_string(),
        }
    }
}

/// Sum `hop.render.Draw`, `src/hop/render.slg:15`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Draw {
    Cube { r#box: crate::hop::geom::Box, color: crate::hop::model::Rgb },
    Wires { r#box: crate::hop::geom::Box, color: crate::hop::model::Rgb },
    Mesh { asset: Asset, at: crate::hop::geom::Vec3, yaw: Int },
}

impl Show for Draw {
    fn show(&self) -> String {
        match self {
            Draw::Cube { r#box, color } => {
                rt::show::record(
                    "Cube",
                    &[("box", r#box.show()), ("color", color.show())],
                )
            }
            Draw::Wires { r#box, color } => {
                rt::show::record(
                    "Wires",
                    &[("box", r#box.show()), ("color", color.show())],
                )
            }
            Draw::Mesh { asset, at, yaw } => {
                rt::show::record(
                    "Mesh",
                    &[("asset", asset.show()), ("at", at.show()), ("yaw", yaw.show())],
                )
            }
        }
    }
}

impl Draw {
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new_cube(
        r#box: crate::hop::geom::Box,
        color: crate::hop::model::Rgb,
    ) -> R<Self> {
        rt::claim(crate::conform::r#box(&r#box)?, "field `box`")?;
        rt::claim(crate::conform::rgb(&color)?, "field `color`")?;
        Ok(Draw::Cube { r#box, color })
    }
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new_wires(
        r#box: crate::hop::geom::Box,
        color: crate::hop::model::Rgb,
    ) -> R<Self> {
        rt::claim(crate::conform::r#box(&r#box)?, "field `box`")?;
        rt::claim(crate::conform::rgb(&color)?, "field `color`")?;
        Ok(Draw::Wires { r#box, color })
    }
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new_mesh(asset: Asset, at: crate::hop::geom::Vec3, yaw: Int) -> R<Self> {
        rt::claim(crate::conform::vec3(&at)?, "field `at`")?;
        rt::claim(crate::conform::range_0_359(&yaw)?, "field `yaw`")?;
        Ok(Draw::Mesh { asset, at, yaw })
    }
}

/// Record `hop.render.Label`, `src/hop/render.slg:17`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Label {
    pub x: Int,
    pub y: Int,
    pub size: Int,
    pub text: String,
    pub color: crate::hop::model::Rgb,
}

impl Show for Label {
    fn show(&self) -> String {
        rt::show::record(
            "Label",
            &[
                ("x", self.x.show()),
                ("y", self.y.show()),
                ("size", self.size.show()),
                ("text", self.text.show()),
                ("color", self.color.show()),
            ],
        )
    }
}

impl Label {
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new(
        x: Int,
        y: Int,
        size: Int,
        text: String,
        color: crate::hop::model::Rgb,
    ) -> R<Self> {
        rt::claim(crate::conform::rgb(&color)?, "field `color`")?;
        Ok(Label { x, y, size, text, color })
    }
}

/// time is the tick count, an exact integer; the shader divides it into seconds
///
/// Record `hop.render.Frame`, `src/hop/render.slg:19`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    pub sky: crate::hop::model::Rgb,
    pub eye: crate::hop::geom::Vec3,
    pub target: crate::hop::geom::Vec3,
    pub time: Int,
    pub world: Vec<Draw>,
    pub labels: Vec<Label>,
}

impl Show for Frame {
    fn show(&self) -> String {
        rt::show::record(
            "Frame",
            &[
                ("sky", self.sky.show()),
                ("eye", self.eye.show()),
                ("target", self.target.show()),
                ("time", self.time.show()),
                ("world", self.world.show()),
                ("labels", self.labels.show()),
            ],
        )
    }
}

impl Frame {
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new(
        sky: crate::hop::model::Rgb,
        eye: crate::hop::geom::Vec3,
        target: crate::hop::geom::Vec3,
        time: Int,
        world: Vec<Draw>,
        labels: Vec<Label>,
    ) -> R<Self> {
        rt::claim(crate::conform::rgb(&sky)?, "field `sky`")?;
        rt::claim(crate::conform::vec3(&eye)?, "field `eye`")?;
        rt::claim(crate::conform::vec3(&target)?, "field `target`")?;
        rt::claim(crate::conform::list_draw(&world)?, "field `world`")?;
        rt::claim(crate::conform::list_label(&labels)?, "field `labels`")?;
        Ok(Frame {
            sky,
            eye,
            target,
            time,
            world,
            labels,
        })
    }
}

/// Constant `hop.render.sky`, `src/hop/render.slg:22`.
pub fn sky() -> R<crate::hop::model::Rgb> {
    crate::hop::model::Rgb::new(135, 206, 235)
}

/// Constant `hop.render.edge`, `src/hop/render.slg:23`.
pub fn edge() -> R<crate::hop::model::Rgb> {
    crate::hop::model::Rgb::new(30, 40, 30)
}

/// Constant `hop.render.ink`, `src/hop/render.slg:24`.
pub fn ink() -> R<crate::hop::model::Rgb> {
    crate::hop::model::Rgb::new(255, 255, 255)
}

/// the camera looks at the hero from the side and a little above, at a fixed distance
///
/// Constant `hop.render.eye_offset`, `src/hop/render.slg:26`.
pub fn eye_offset() -> R<crate::hop::geom::Vec3> {
    crate::hop::geom::Vec3::new(0, 512, 2048)
}

/// Constant `hop.render.look_height`, `src/hop/render.slg:28`.
pub const LOOK_HEIGHT: Int = 192;

/// Fn `hop.render.asset_name`, `src/hop/render.slg:30`.
pub fn asset_name(a: Asset) -> R<String> {
    Ok(
        match &a {
            Asset::PipMesh => String::from("pip"),
            Asset::CrawlerMesh => String::from("crawler"),
            Asset::GemMesh => String::from("gem"),
            Asset::FlagMesh => String::from("flag"),
        },
    )
}

/// a gem bobs a quarter of a metre and turns three degrees a tick
///
/// Fn `hop.render.bob`, `src/hop/render.slg:38`.
pub fn bob(t: Int) -> R<Int> {
    rt::int::sub(rt::int::mul(crate::hop::geom::triangle(t, 32)?, 2)?, 32)
}

/// Fn `hop.render.block`, `src/hop/render.slg:42`.
pub fn block(
    r#box: crate::hop::geom::Box,
    color: crate::hop::model::Rgb,
) -> R<Vec<Draw>> {
    rt::claim(crate::conform::r#box(&r#box)?, "argument `box` of hop.render.block")?;
    rt::claim(crate::conform::rgb(&color)?, "argument `color` of hop.render.block")?;
    let result: Vec<Draw> = vec![
        Draw::new_cube(r#box.clone(), color.clone())?,
        Draw::new_wires(r#box.clone(), edge()?)?,
    ];
    rt::claim(crate::conform::list_draw(&result)?, "the result of hop.render.block")?;
    Ok(result)
}

/// Fn `hop.render.banner`, `src/hop/render.slg:44`.
pub fn banner(m_: crate::hop::model::Mode) -> R<Vec<Label>> {
    let result: Vec<Label> = match &m_ {
        crate::hop::model::Mode::Playing => Vec::<Label>::new(),
        crate::hop::model::Mode::Won => {
            vec![Label::new(380, 250, 40, String::from("level clear"), ink()?)?]
        }
        crate::hop::model::Mode::Lost => {
            vec![Label::new(400, 250, 40, String::from("game over"), ink()?)?]
        }
    };
    rt::claim(crate::conform::list_label(&result)?, "the result of hop.render.banner")?;
    Ok(result)
}

/// solids, then movers, gems, crawlers, the flag and last the hero, so the order of the frame is fixed
///
/// Fn `hop.render.scene`, `src/hop/render.slg:50`.
pub fn scene(v: crate::hop::world::View) -> R<Frame> {
    rt::claim(crate::conform::view(&v)?, "argument `v` of hop.render.scene")?;
    let result: Frame = {
        let target: crate::hop::geom::Vec3 = crate::hop::geom::plus(
            v.player.at.clone(),
            crate::hop::geom::Vec3::new(0, LOOK_HEIGHT, 0)?,
        )?;
        let mut draws: Vec<Draw> = Vec::<Draw>::new();
        rt::claim(crate::conform::list_draw(&draws)?, "`draws`")?;
        for s in v.stage.solids.clone() {
            draws = rt::list::concat(&draws, &block(s.r#box.clone(), s.color.clone())?);
        }
        for m_ in v.stage.movers.clone() {
            draws = rt::list::concat(
                &draws,
                &block(
                    crate::hop::physics::mover_box(m_.clone(), v.ticks)?,
                    m_.color.clone(),
                )?,
            );
        }
        for g in rt::list::filter(
            &v.stage.gems,
            |g: crate::hop::model::Gem| -> R<bool> { Ok(!v.taken.contains(&g.no)) },
        )? {
            let mut tmp1 = g.at.clone();
            let tmp2 = rt::int::add(g.at.y, bob(v.ticks)?)?;
            rt::claim(crate::conform::sub(&tmp2)?, "field `y`")?;
            tmp1.y = tmp2;
            let tmp3 = Draw::new_mesh(
                Asset::GemMesh,
                tmp1,
                rt::int::modulo(rt::int::mul(v.ticks, 3)?, 360)?,
            )?;
            draws.push(tmp3);
        }
        for c in v.crawlers.clone() {
            let tmp4 = Draw::new_mesh(
                Asset::CrawlerMesh,
                c.at.clone(),
                if c.dir < 0 { 180 } else { 0 },
            )?;
            draws.push(tmp4);
        }
        let tmp5 = Draw::new_mesh(Asset::FlagMesh, v.stage.goal.clone(), 0)?;
        draws.push(tmp5);
        let tmp6 = Draw::new_mesh(Asset::PipMesh, v.player.at.clone(), v.player.yaw)?;
        draws.push(tmp6);
        let hud: Vec<Label> = vec![
            Label::new(
                16,
                12,
                20,
                format!(
                    "gems {}/{}",
                    rt::int::count(v.taken.len()).show(),
                    rt::int::count(v.stage.gems.len()).show(),
                ),
                ink()?,
            )?,
            Label::new(16, 38, 20, format!("lives {}", v.lives.show()), ink()?)?,
        ];
        Frame::new(
            sky()?,
            crate::hop::geom::plus(target.clone(), eye_offset()?)?,
            target.clone(),
            v.ticks,
            draws.clone(),
            rt::list::concat(&hud, &banner(v.mode.clone())?),
        )?
    };
    rt::claim(crate::conform::frame(&result)?, "the result of hop.render.scene")?;
    Ok(result)
}

/// Fn `hop.render.rgb_text`, `src/hop/render.slg:69`.
pub fn rgb_text(c: crate::hop::model::Rgb) -> R<String> {
    rt::claim(crate::conform::rgb(&c)?, "argument `c` of hop.render.rgb_text")?;
    Ok(format!("{},{},{}", c.r.show(), c.g.show(), c.b.show()))
}

/// Fn `hop.render.draw_text`, `src/hop/render.slg:71`.
pub fn draw_text(d: Draw) -> R<String> {
    rt::claim(crate::conform::draw(&d)?, "argument `d` of hop.render.draw_text")?;
    Ok(
        match &d {
            Draw::Cube { r#box: tmp1, color: tmp2, .. } => {
                let b = (*tmp1).clone();
                let color = (*tmp2).clone();
                format!(
                    "cube {} {} {}",
                    crate::hop::geom::vec_text(b.lo.clone())?,
                    crate::hop::geom::vec_text(b.hi.clone())?,
                    rgb_text(color.clone())?,
                )
            }
            Draw::Wires { r#box: tmp3, color: tmp4, .. } => {
                let b = (*tmp3).clone();
                let color = (*tmp4).clone();
                format!(
                    "wires {} {} {}",
                    crate::hop::geom::vec_text(b.lo.clone())?,
                    crate::hop::geom::vec_text(b.hi.clone())?,
                    rgb_text(color.clone())?,
                )
            }
            Draw::Mesh { asset: tmp5, at: tmp6, yaw: tmp7, .. } => {
                let asset = (*tmp5).clone();
                let at = (*tmp6).clone();
                let yaw = *tmp7;
                format!(
                    "mesh {} {} {}",
                    asset_name(asset.clone())?,
                    crate::hop::geom::vec_text(at.clone())?,
                    yaw.show(),
                )
            }
        },
    )
}

/// a frame as text, one line per call the screen will make, so four backends can be compared without a GPU
///
/// Fn `hop.render.frame_lines`, `src/hop/render.slg:81`.
pub fn frame_lines(f: Frame) -> R<Vec<String>> {
    rt::claim(crate::conform::frame(&f)?, "argument `f` of hop.render.frame_lines")?;
    Ok(
        rt::list::concat(
            &rt::list::concat(
                &[
                    format!("sky {}", rgb_text(f.sky.clone())?),
                    format!(
                        "camera {} {}",
                        crate::hop::geom::vec_text(f.eye.clone())?,
                        crate::hop::geom::vec_text(f.target.clone())?,
                    ),
                    format!("time {}", f.time.show()),
                ],
                &rt::list::map(
                    &f.world,
                    |d: Draw| -> R<String> { draw_text(d.clone()) },
                )?,
            ),
            &rt::list::map(
                &f.labels,
                |l: Label| -> R<String> {
                    Ok(
                        format!(
                            "label {} {} {} {} {}",
                            l.x.show(),
                            l.y.show(),
                            l.size.show(),
                            rgb_text(l.color.clone())?,
                            l.text,
                        ),
                    )
                },
            )?,
        ),
    )
}

/// Fn `hop.render.dump`, `src/hop/render.slg:87`.
pub fn dump(m: &mut Machine) -> R<Vec<String>> {
    frame_lines(scene(crate::hop::world::view(m)?)?)
}
