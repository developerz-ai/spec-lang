//! Module `hop.geom`, printed by spec-lang from `src/hop/geom.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.

use crate::rt::{self, Int, R, Show};

/// 4096 m either way from the origin, so every coordinate is exact as an f32 at the C edge
///
/// Type `hop.geom.Sub`, `src/hop/geom.slg:3`.
/// Checked against `-1048576..=1048576` wherever a value of it is given.
pub type Sub = Int;

/// Record `hop.geom.Vec3`, `src/hop/geom.slg:9`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Vec3 {
    pub x: Int,
    pub y: Int,
    pub z: Int,
}

impl Show for Vec3 {
    fn show(&self) -> String {
        rt::show::record(
            "Vec3",
            &[("x", self.x.show()), ("y", self.y.show()), ("z", self.z.show())],
        )
    }
}

impl Vec3 {
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new(x: Int, y: Int, z: Int) -> R<Self> {
        rt::claim(crate::conform::sub(&x)?, "field `x`")?;
        rt::claim(crate::conform::sub(&y)?, "field `y`")?;
        rt::claim(crate::conform::sub(&z)?, "field `z`")?;
        Ok(Vec3 { x, y, z })
    }
}

/// lo is the corner nearest minus infinity on every axis
///
/// Record `hop.geom.Box`, `src/hop/geom.slg:11`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Box {
    pub lo: Vec3,
    pub hi: Vec3,
}

impl Show for Box {
    fn show(&self) -> String {
        rt::show::record("Box", &[("lo", self.lo.show()), ("hi", self.hi.show())])
    }
}

impl Box {
    /// Builds one, checking every field in order; a field left out (`None`) takes its default.
    pub fn new(lo: Vec3, hi: Vec3) -> R<Self> {
        rt::claim(crate::conform::vec3(&lo)?, "field `lo`")?;
        rt::claim(crate::conform::vec3(&hi)?, "field `hi`")?;
        Ok(Box { lo, hi })
    }
}

/// 256 sub-metres to the metre: a power of two, so metres are exact binary fractions
///
/// Constant `hop.geom.unit`, `src/hop/geom.slg:6`.
pub const UNIT: Int = 256;

/// Fn `hop.geom.plus`, `src/hop/geom.slg:14`.
pub fn plus(a: Vec3, b: Vec3) -> R<Vec3> {
    rt::claim(crate::conform::vec3(&a)?, "argument `a` of hop.geom.plus")?;
    rt::claim(crate::conform::vec3(&b)?, "argument `b` of hop.geom.plus")?;
    let result: Vec3 = Vec3::new(
        rt::int::add(a.x, b.x)?,
        rt::int::add(a.y, b.y)?,
        rt::int::add(a.z, b.z)?,
    )?;
    rt::claim(crate::conform::vec3(&result)?, "the result of hop.geom.plus")?;
    Ok(result)
}

/// Fn `hop.geom.minus`, `src/hop/geom.slg:17`.
pub fn minus(a: Vec3, b: Vec3) -> R<Vec3> {
    rt::claim(crate::conform::vec3(&a)?, "argument `a` of hop.geom.minus")?;
    rt::claim(crate::conform::vec3(&b)?, "argument `b` of hop.geom.minus")?;
    let result: Vec3 = Vec3::new(
        rt::int::sub(a.x, b.x)?,
        rt::int::sub(a.y, b.y)?,
        rt::int::sub(a.z, b.z)?,
    )?;
    rt::claim(crate::conform::vec3(&result)?, "the result of hop.geom.minus")?;
    Ok(result)
}

/// Fn `hop.geom.scaled`, `src/hop/geom.slg:19`.
pub fn scaled(v: Vec3, k: Int) -> R<Vec3> {
    rt::claim(crate::conform::vec3(&v)?, "argument `v` of hop.geom.scaled")?;
    let result: Vec3 = Vec3::new(
        rt::int::mul(v.x, k)?,
        rt::int::mul(v.y, k)?,
        rt::int::mul(v.z, k)?,
    )?;
    rt::claim(crate::conform::vec3(&result)?, "the result of hop.geom.scaled")?;
    Ok(result)
}

/// Fn `hop.geom.shifted`, `src/hop/geom.slg:21`.
pub fn shifted(b: Box, by: Vec3) -> R<Box> {
    rt::claim(crate::conform::r#box(&b)?, "argument `b` of hop.geom.shifted")?;
    rt::claim(crate::conform::vec3(&by)?, "argument `by` of hop.geom.shifted")?;
    let result: Box = Box::new(
        plus(b.lo.clone(), by.clone())?,
        plus(b.hi.clone(), by.clone())?,
    )?;
    rt::claim(crate::conform::r#box(&result)?, "the result of hop.geom.shifted")?;
    Ok(result)
}

/// a body is placed by the centre of its base: its feet
///
/// Fn `hop.geom.around_feet`, `src/hop/geom.slg:23`.
pub fn around_feet(feet: Vec3, half: Vec3) -> R<Box> {
    rt::claim(crate::conform::vec3(&feet)?, "argument `feet` of hop.geom.around_feet")?;
    rt::claim(crate::conform::vec3(&half)?, "argument `half` of hop.geom.around_feet")?;
    let result: Box = Box::new(
        Vec3::new(rt::int::sub(feet.x, half.x)?, feet.y, rt::int::sub(feet.z, half.z)?)?,
        Vec3::new(
            rt::int::add(feet.x, half.x)?,
            rt::int::add(feet.y, half.y)?,
            rt::int::add(feet.z, half.z)?,
        )?,
    )?;
    rt::claim(crate::conform::r#box(&result)?, "the result of hop.geom.around_feet")?;
    Ok(result)
}

/// a gem is placed by its centre
///
/// Fn `hop.geom.around_centre`, `src/hop/geom.slg:28`.
pub fn around_centre(c: Vec3, half: Int) -> R<Box> {
    rt::claim(crate::conform::vec3(&c)?, "argument `c` of hop.geom.around_centre")?;
    let result: Box = Box::new(
        Vec3::new(
            rt::int::sub(c.x, half)?,
            rt::int::sub(c.y, half)?,
            rt::int::sub(c.z, half)?,
        )?,
        Vec3::new(
            rt::int::add(c.x, half)?,
            rt::int::add(c.y, half)?,
            rt::int::add(c.z, half)?,
        )?,
    )?;
    rt::claim(crate::conform::r#box(&result)?, "the result of hop.geom.around_centre")?;
    Ok(result)
}

/// solids block only when they share volume: a body standing on a floor touches it and is not inside it
///
/// Fn `hop.geom.overlaps`, `src/hop/geom.slg:32`.
pub fn overlaps(a: Box, b: Box) -> R<bool> {
    rt::claim(crate::conform::r#box(&a)?, "argument `a` of hop.geom.overlaps")?;
    rt::claim(crate::conform::r#box(&b)?, "argument `b` of hop.geom.overlaps")?;
    Ok(
        (((((a.lo.x < b.hi.x) && (b.lo.x < a.hi.x)) && (a.lo.y < b.hi.y))
            && (b.lo.y < a.hi.y)) && (a.lo.z < b.hi.z)) && (b.lo.z < a.hi.z),
    )
}

/// pickups and hits count a touch, faces included: the same test raylib's CheckCollisionBoxes makes
///
/// Fn `hop.geom.touches`, `src/hop/geom.slg:40`.
pub fn touches(a: Box, b: Box) -> R<bool> {
    rt::claim(crate::conform::r#box(&a)?, "argument `a` of hop.geom.touches")?;
    rt::claim(crate::conform::r#box(&b)?, "argument `b` of hop.geom.touches")?;
    Ok(
        (((((a.lo.x <= b.hi.x) && (b.lo.x <= a.hi.x)) && (a.lo.y <= b.hi.y))
            && (b.lo.y <= a.hi.y)) && (a.lo.z <= b.hi.z)) && (b.lo.z <= a.hi.z),
    )
}

/// the footprints meet strictly: standing needs some floor under the feet, not an edge
///
/// Fn `hop.geom.over`, `src/hop/geom.slg:48`.
pub fn over(a: Box, b: Box) -> R<bool> {
    rt::claim(crate::conform::r#box(&a)?, "argument `a` of hop.geom.over")?;
    rt::claim(crate::conform::r#box(&b)?, "argument `b` of hop.geom.over")?;
    Ok(
        (((a.lo.x < b.hi.x) && (b.lo.x < a.hi.x)) && (a.lo.z < b.hi.z))
            && (b.lo.z < a.hi.z),
    )
}

/// back and forth along a path without division: 0 up to n, then down to 0, period 2n
///
/// Fn `hop.geom.triangle`, `src/hop/geom.slg:51`.
pub fn triangle(t: Int, n: Int) -> R<Int> {
    let p: Int = rt::int::modulo(t, rt::int::mul(2, n)?)?;
    Ok(if p <= n { p } else { rt::int::sub(rt::int::mul(2, n)?, p)? })
}

/// Fn `hop.geom.approach`, `src/hop/geom.slg:58`.
pub fn approach(v: Int, target: Int, step: Int) -> R<Int> {
    Ok(
        if v < target {
            rt::int::add(v, step)?.min(target)
        } else {
            rt::int::sub(v, step)?.max(target)
        },
    )
}

/// Fn `hop.geom.vec_text`, `src/hop/geom.slg:62`.
pub fn vec_text(v: Vec3) -> R<String> {
    rt::claim(crate::conform::vec3(&v)?, "argument `v` of hop.geom.vec_text")?;
    Ok(format!("{},{},{}", v.x.show(), v.y.show(), v.z.show()))
}
