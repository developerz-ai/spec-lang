//! Module `hop.physics`, printed by spec-lang from `src/hop/physics.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.

use crate::rt::{self, Int, Opt, R, Show};

/// Record `hop.physics.Axis`, `src/hop/physics.slg:35`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Axis {
    pub pos: Int,
    pub vel: Int,
}

impl Show for Axis {
    fn show(&self) -> String {
        rt::show::record("Axis", &[("pos", self.pos.show()), ("vel", self.vel.show())])
    }
}

/// Record `hop.physics.Fall`, `src/hop/physics.slg:36`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fall {
    pub y: Int,
    pub vel: Int,
    pub grounded: bool,
    pub riding: Opt<Int>,
}

impl Show for Fall {
    fn show(&self) -> String {
        rt::show::record(
            "Fall",
            &[
                ("y", self.y.show()),
                ("vel", self.vel.show()),
                ("grounded", self.grounded.show()),
                ("riding", self.riding.show()),
            ],
        )
    }
}

/// Record `hop.physics.Placed`, `src/hop/physics.slg:37`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Placed {
    pub no: Int,
    pub r#box: crate::hop::geom::Box,
}

impl Show for Placed {
    fn show(&self) -> String {
        rt::show::record("Placed", &[("no", self.no.show()), ("box", self.r#box.show())])
    }
}

impl Placed {
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new(no: Int, r#box: crate::hop::geom::Box) -> R<Self> {
        rt::claim(crate::conform::r#box(&r#box)?, "field `box`")?;
        Ok(Placed { no, r#box })
    }
}

/// half the width, the full height and half the depth of the hero: 0.5 m by 0.75 m by 0.5 m
///
/// Constant `hop.physics.body_half`, `src/hop/physics.slg:11`.
pub fn body_half() -> R<crate::hop::geom::Vec3> {
    crate::hop::geom::Vec3::new(64, 192, 64)
}

/// Constant `hop.physics.crawler_half`, `src/hop/physics.slg:13`.
pub fn crawler_half() -> R<crate::hop::geom::Vec3> {
    crate::hop::geom::Vec3::new(96, 128, 96)
}

/// Constant `hop.physics.gem_half`, `src/hop/physics.slg:14`.
pub const GEM_HALF: Int = 64;

/// Constant `hop.physics.goal_half`, `src/hop/physics.slg:15`.
pub fn goal_half() -> R<crate::hop::geom::Vec3> {
    crate::hop::geom::Vec3::new(64, 768, 64)
}

/// 3.75 m/s flat out; about the same speed on a diagonal, so a diagonal is not a shortcut
///
/// Constant `hop.physics.run_speed`, `src/hop/physics.slg:17`.
pub const RUN_SPEED: Int = 16;

/// Constant `hop.physics.diagonal_speed`, `src/hop/physics.slg:19`.
pub const DIAGONAL_SPEED: Int = 11;

/// Constant `hop.physics.ground_accel`, `src/hop/physics.slg:20`.
pub const GROUND_ACCEL: Int = 2;

/// Constant `hop.physics.air_accel`, `src/hop/physics.slg:21`.
pub const AIR_ACCEL: Int = 1;

/// Constant `hop.physics.gravity`, `src/hop/physics.slg:22`.
pub const GRAVITY: Int = 2;

/// never more than 48 sub-metres in a tick, less than the thinnest solid, so nothing passes through a floor
///
/// Constant `hop.physics.max_fall`, `src/hop/physics.slg:23`.
pub const MAX_FALL: Int = 48;

/// Constant `hop.physics.jump_speed`, `src/hop/physics.slg:25`.
pub const JUMP_SPEED: Int = 40;

/// letting go of jump early cuts the rise: a short hop
///
/// Constant `hop.physics.hop_cut`, `src/hop/physics.slg:26`.
pub const HOP_CUT: Int = 16;

/// a jump still counts for a few ticks after running off an edge
///
/// Constant `hop.physics.coyote_ticks`, `src/hop/physics.slg:28`.
pub const COYOTE_TICKS: Int = 6;

/// Constant `hop.physics.stomp_bounce`, `src/hop/physics.slg:30`.
pub const STOMP_BOUNCE: Int = 24;

/// a stomp lands on the upper half of a crawler
///
/// Constant `hop.physics.stomp_line`, `src/hop/physics.slg:31`.
pub const STOMP_LINE: Int = 64;

/// Constant `hop.physics.kill_y`, `src/hop/physics.slg:33`.
pub fn kill_y() -> R<Int> {
    rt::int::neg(2048)
}

/// Fn `hop.physics.body`, `src/hop/physics.slg:39`.
pub fn body(at: crate::hop::geom::Vec3) -> R<crate::hop::geom::Box> {
    rt::claim(crate::conform::vec3(&at)?, "argument `at` of hop.physics.body")?;
    let result: crate::hop::geom::Box = crate::hop::geom::around_feet(
        at.clone(),
        body_half()?,
    )?;
    rt::claim(crate::conform::r#box(&result)?, "the result of hop.physics.body")?;
    Ok(result)
}

/// Fn `hop.physics.crawler_box`, `src/hop/physics.slg:41`.
pub fn crawler_box(c: crate::hop::model::Crawler) -> R<crate::hop::geom::Box> {
    rt::claim(crate::conform::crawler(&c)?, "argument `c` of hop.physics.crawler_box")?;
    let result: crate::hop::geom::Box = crate::hop::geom::around_feet(
        c.at.clone(),
        crawler_half()?,
    )?;
    rt::claim(crate::conform::r#box(&result)?, "the result of hop.physics.crawler_box")?;
    Ok(result)
}

/// Fn `hop.physics.mover_box`, `src/hop/physics.slg:43`.
pub fn mover_box(m_: crate::hop::model::Mover, t: Int) -> R<crate::hop::geom::Box> {
    rt::claim(crate::conform::mover(&m_)?, "argument `m` of hop.physics.mover_box")?;
    let result: crate::hop::geom::Box = crate::hop::geom::shifted(
        m_.home.clone(),
        crate::hop::geom::scaled(
            m_.step.clone(),
            crate::hop::geom::triangle(t, m_.span)?,
        )?,
    )?;
    rt::claim(crate::conform::r#box(&result)?, "the result of hop.physics.mover_box")?;
    Ok(result)
}

/// a rider moves exactly as far as its mover did this tick
///
/// Fn `hop.physics.mover_shift`, `src/hop/physics.slg:45`.
pub fn mover_shift(m_: crate::hop::model::Mover, t: Int) -> R<crate::hop::geom::Vec3> {
    rt::claim(crate::conform::mover(&m_)?, "argument `m` of hop.physics.mover_shift")?;
    let result: crate::hop::geom::Vec3 = crate::hop::geom::minus(
        mover_box(m_.clone(), t)?.lo,
        mover_box(m_.clone(), rt::int::sub(t, 1)?)?.lo,
    )?;
    rt::claim(crate::conform::vec3(&result)?, "the result of hop.physics.mover_shift")?;
    Ok(result)
}

/// Fn `hop.physics.axis`, `src/hop/physics.slg:48`.
pub fn axis(neg: bool, pos: bool) -> R<Int> {
    rt::int::sub(if pos { 1 } else { 0 }, if neg { 1 } else { 0 })
}

/// x runs right, z runs toward the camera, so forward is minus z
///
/// Fn `hop.physics.walk_target`, `src/hop/physics.slg:50`.
pub fn walk_target(pad: crate::hop::model::Pad) -> R<crate::hop::geom::Vec3> {
    let result: crate::hop::geom::Vec3 = {
        let dx: Int = axis(pad.left, pad.right)?;
        let dz: Int = axis(pad.forward, pad.back)?;
        let speed: Int = if (dx != 0) && (dz != 0) { DIAGONAL_SPEED } else { RUN_SPEED };
        crate::hop::geom::Vec3::new(
            rt::int::mul(dx, speed)?,
            0,
            rt::int::mul(dz, speed)?,
        )?
    };
    rt::claim(crate::conform::vec3(&result)?, "the result of hop.physics.walk_target")?;
    Ok(result)
}

/// the hero faces where the pad points, and keeps facing there when it lets go
///
/// Fn `hop.physics.facing`, `src/hop/physics.slg:59`.
pub fn facing(pad: crate::hop::model::Pad, was: Int) -> R<Int> {
    let dx: Int = axis(pad.left, pad.right)?;
    let dz: Int = axis(pad.forward, pad.back)?;
    if dx == 1 {
        return Ok(
            if dz == rt::int::neg(1)? { 45 } else { if dz == 1 { 315 } else { 0 } },
        );
    }
    if dx == rt::int::neg(1)? {
        return Ok(
            if dz == rt::int::neg(1)? { 135 } else { if dz == 1 { 225 } else { 180 } },
        );
    }
    Ok(if dz == rt::int::neg(1)? { 90 } else { if dz == 1 { 270 } else { was } })
}

/// move on one axis, then stop flush against the first solid in the way
///
/// Fn `hop.physics.slide_x`, `src/hop/physics.slg:69`.
pub fn slide_x(
    at: crate::hop::geom::Vec3,
    v: Int,
    solids: Vec<crate::hop::model::Solid>,
) -> R<Axis> {
    rt::claim(crate::conform::vec3(&at)?, "argument `at` of hop.physics.slide_x")?;
    rt::claim(
        crate::conform::list_solid(&solids)?,
        "argument `solids` of hop.physics.slide_x",
    )?;
    if v == 0 {
        return Ok(Axis { pos: at.x, vel: 0 });
    }
    let hits: Vec<crate::hop::model::Solid> = rt::list::filter(
        &solids,
        |s: crate::hop::model::Solid| -> R<bool> {
            let mut tmp1 = at.clone();
            let tmp2 = rt::int::add(at.x, v)?;
            rt::claim(crate::conform::sub(&tmp2)?, "field `x`")?;
            tmp1.x = tmp2;
            crate::hop::geom::overlaps(body(tmp1)?, s.r#box.clone())
        },
    )?;
    if rt::int::count(hits.len()) == 0 {
        return Ok(Axis {
            pos: rt::int::add(at.x, v)?,
            vel: v,
        });
    }
    if v > 0 {
        return Ok(Axis {
            pos: rt::int::sub(
                rt::list::min(
                    &rt::list::map(
                        &hits,
                        |it: crate::hop::model::Solid| -> R<Int> { Ok(it.r#box.lo.x) },
                    )?,
                )?,
                body_half()?.x,
            )?,
            vel: 0,
        });
    }
    Ok(Axis {
        pos: rt::int::add(
            rt::list::max(
                &rt::list::map(
                    &hits,
                    |it: crate::hop::model::Solid| -> R<Int> { Ok(it.r#box.hi.x) },
                )?,
            )?,
            body_half()?.x,
        )?,
        vel: 0,
    })
}

/// Fn `hop.physics.slide_z`, `src/hop/physics.slg:78`.
pub fn slide_z(
    at: crate::hop::geom::Vec3,
    v: Int,
    solids: Vec<crate::hop::model::Solid>,
) -> R<Axis> {
    rt::claim(crate::conform::vec3(&at)?, "argument `at` of hop.physics.slide_z")?;
    rt::claim(
        crate::conform::list_solid(&solids)?,
        "argument `solids` of hop.physics.slide_z",
    )?;
    if v == 0 {
        return Ok(Axis { pos: at.z, vel: 0 });
    }
    let hits: Vec<crate::hop::model::Solid> = rt::list::filter(
        &solids,
        |s: crate::hop::model::Solid| -> R<bool> {
            let mut tmp1 = at.clone();
            let tmp2 = rt::int::add(at.z, v)?;
            rt::claim(crate::conform::sub(&tmp2)?, "field `z`")?;
            tmp1.z = tmp2;
            crate::hop::geom::overlaps(body(tmp1)?, s.r#box.clone())
        },
    )?;
    if rt::int::count(hits.len()) == 0 {
        return Ok(Axis {
            pos: rt::int::add(at.z, v)?,
            vel: v,
        });
    }
    if v > 0 {
        return Ok(Axis {
            pos: rt::int::sub(
                rt::list::min(
                    &rt::list::map(
                        &hits,
                        |it: crate::hop::model::Solid| -> R<Int> { Ok(it.r#box.lo.z) },
                    )?,
                )?,
                body_half()?.z,
            )?,
            vel: 0,
        });
    }
    Ok(Axis {
        pos: rt::int::add(
            rt::list::max(
                &rt::list::map(
                    &hits,
                    |it: crate::hop::model::Solid| -> R<Int> { Ok(it.r#box.hi.z) },
                )?,
            )?,
            body_half()?.z,
        )?,
        vel: 0,
    })
}

/// solids stop the body both ways; a mover's top only catches feet that were above it
///
/// Fn `hop.physics.fall`, `src/hop/physics.slg:86`.
pub fn fall(
    at: crate::hop::geom::Vec3,
    v: Int,
    solids: Vec<crate::hop::model::Solid>,
    tops: Vec<Placed>,
) -> R<Fall> {
    rt::claim(crate::conform::vec3(&at)?, "argument `at` of hop.physics.fall")?;
    rt::claim(
        crate::conform::list_solid(&solids)?,
        "argument `solids` of hop.physics.fall",
    )?;
    rt::claim(
        crate::conform::list_placed(&tops)?,
        "argument `tops` of hop.physics.fall",
    )?;
    let hits: Vec<crate::hop::model::Solid> = rt::list::filter(
        &solids,
        |s: crate::hop::model::Solid| -> R<bool> {
            let mut tmp1 = at.clone();
            let tmp2 = rt::int::add(at.y, v)?;
            rt::claim(crate::conform::sub(&tmp2)?, "field `y`")?;
            tmp1.y = tmp2;
            crate::hop::geom::overlaps(body(tmp1)?, s.r#box.clone())
        },
    )?;
    if (rt::int::count(hits.len()) > 0) && (v > 0) {
        return Ok(Fall {
            y: rt::int::sub(
                rt::list::min(
                    &rt::list::map(
                        &hits,
                        |it: crate::hop::model::Solid| -> R<Int> { Ok(it.r#box.lo.y) },
                    )?,
                )?,
                body_half()?.y,
            )?,
            vel: 0,
            grounded: false,
            riding: Opt::None,
        });
    }
    if rt::int::count(hits.len()) > 0 {
        return Ok(Fall {
            y: rt::list::max(
                &rt::list::map(
                    &hits,
                    |it: crate::hop::model::Solid| -> R<Int> { Ok(it.r#box.hi.y) },
                )?,
            )?,
            vel: 0,
            grounded: true,
            riding: Opt::None,
        });
    }
    let catching: Vec<Placed> = rt::list::filter(
        &tops,
        |p: Placed| -> R<bool> {
            Ok(
                (((v < 0) && crate::hop::geom::over(body(at.clone())?, p.r#box.clone())?)
                    && (at.y >= p.r#box.hi.y)) && (rt::int::add(at.y, v)? < p.r#box.hi.y),
            )
        },
    )?;
    if rt::int::count(catching.len()) == 0 {
        return Ok(Fall {
            y: rt::int::add(at.y, v)?,
            vel: v,
            grounded: false,
            riding: Opt::None,
        });
    }
    let top: Int = rt::list::max(
        &rt::list::map(&catching, |it: Placed| -> R<Int> { Ok(it.r#box.hi.y) })?,
    )?;
    Ok(Fall {
        y: top,
        vel: 0,
        grounded: true,
        riding: Opt::Some(
            rt::list::min(
                &rt::list::map(
                    &rt::list::filter(
                        &catching,
                        |it: Placed| -> R<bool> { Ok(it.r#box.hi.y == top) },
                    )?,
                    |it: Placed| -> R<Int> { Ok(it.no) },
                )?,
            )?,
        ),
    })
}

/// a crawler walks to the end of its beat and turns
///
/// Fn `hop.physics.patrol`, `src/hop/physics.slg:97`.
pub fn patrol(c: crate::hop::model::Crawler) -> R<crate::hop::model::Crawler> {
    rt::claim(crate::conform::crawler(&c)?, "argument `c` of hop.physics.patrol")?;
    let result: crate::hop::model::Crawler = 'b1: {
        let x: Int = rt::int::add(c.at.x, rt::int::mul(c.dir, c.speed)?)?;
        if x >= c.hi {
            let tmp3 = c.clone();
            let mut tmp1 = c.at.clone();
            let tmp2 = c.hi;
            rt::claim(crate::conform::sub(&tmp2)?, "field `x`")?;
            tmp1.x = tmp2;
            let mut tmp4 = tmp3;
            let tmp5 = tmp1;
            rt::claim(crate::conform::vec3(&tmp5)?, "field `at`")?;
            tmp4.at = tmp5;
            let tmp6 = rt::int::neg(1)?;
            rt::claim(crate::conform::range_minus1_1(&tmp6)?, "field `dir`")?;
            tmp4.dir = tmp6;
            break 'b1 tmp4;
        }
        if x <= c.lo {
            let tmp9 = c.clone();
            let mut tmp7 = c.at.clone();
            let tmp8 = c.lo;
            rt::claim(crate::conform::sub(&tmp8)?, "field `x`")?;
            tmp7.x = tmp8;
            let mut tmp10 = tmp9;
            let tmp11 = tmp7;
            rt::claim(crate::conform::vec3(&tmp11)?, "field `at`")?;
            tmp10.at = tmp11;
            let tmp12 = 1;
            rt::claim(crate::conform::range_minus1_1(&tmp12)?, "field `dir`")?;
            tmp10.dir = tmp12;
            break 'b1 tmp10;
        }
        let tmp15 = c.clone();
        let mut tmp13 = c.at.clone();
        let tmp14 = x;
        rt::claim(crate::conform::sub(&tmp14)?, "field `x`")?;
        tmp13.x = tmp14;
        let mut tmp16 = tmp15;
        let tmp17 = tmp13;
        rt::claim(crate::conform::vec3(&tmp17)?, "field `at`")?;
        tmp16.at = tmp17;
        tmp16
    };
    rt::claim(crate::conform::crawler(&result)?, "the result of hop.physics.patrol")?;
    Ok(result)
}

/// from above and falling, on the upper half: a stomp; any other touch hurts
///
/// Fn `hop.physics.stomps`, `src/hop/physics.slg:109`.
pub fn stomps(feet: Int, vy: Int, c: crate::hop::model::Crawler) -> R<bool> {
    rt::claim(crate::conform::crawler(&c)?, "argument `c` of hop.physics.stomps")?;
    Ok((vy < 0) && (feet >= rt::int::add(c.at.y, STOMP_LINE)?))
}
