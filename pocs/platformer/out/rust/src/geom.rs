// Projected by hand from Sub, unit, Vec3, Box, plus, minus, scaled, shifted, around_feet, around_centre, overlaps, touches, over, triangle, approach, vec_text (src/hop/geom.slg). Stand-in for spec-lang build output; change the spec, not this file.
//! Points and boxes in sub-metres. Pure.

use std::fmt;

/// `type Sub = -1048576..=1048576`. Held in an `i32`: contract section 1 bounds every value this program reaches below 2^31.
pub type Sub = i32;

/// `const unit: Int = 256`: sub-metres to the metre.
pub const UNIT: i32 = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct Vec3 {
    pub x: Sub,
    pub y: Sub,
    pub z: Sub,
}

impl Vec3 {
    pub const fn new(x: Sub, y: Sub, z: Sub) -> Self {
        Self { x, y, z }
    }

    pub const fn with_x(self, x: Sub) -> Self {
        Self { x, ..self }
    }

    pub const fn with_y(self, y: Sub) -> Self {
        Self { y, ..self }
    }

    pub const fn with_z(self, z: Sub) -> Self {
        Self { z, ..self }
    }
}

/// `vec_text`: `"{x},{y},{z}"`, written straight into the formatter.
impl fmt::Display for Vec3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{},{},{}", self.x, self.y, self.z)
    }
}

/// `lo` is the corner nearest minus infinity on every axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Box {
    pub lo: Vec3,
    pub hi: Vec3,
}

impl Box {
    pub const fn new(lo: Vec3, hi: Vec3) -> Self {
        Self { lo, hi }
    }
}

pub const fn plus(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

pub const fn minus(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

pub const fn scaled(v: Vec3, k: i32) -> Vec3 {
    Vec3::new(v.x * k, v.y * k, v.z * k)
}

pub const fn shifted(b: Box, by: Vec3) -> Box {
    Box::new(plus(b.lo, by), plus(b.hi, by))
}

/// A body is placed by the centre of its base: its feet.
pub const fn around_feet(feet: Vec3, half: Vec3) -> Box {
    Box::new(
        Vec3::new(feet.x - half.x, feet.y, feet.z - half.z),
        Vec3::new(feet.x + half.x, feet.y + half.y, feet.z + half.z),
    )
}

/// A gem is placed by its centre.
pub const fn around_centre(c: Vec3, half: i32) -> Box {
    Box::new(
        Vec3::new(c.x - half, c.y - half, c.z - half),
        Vec3::new(c.x + half, c.y + half, c.z + half),
    )
}

/// Solids block only when they share volume.
pub const fn overlaps(a: Box, b: Box) -> bool {
    a.lo.x < b.hi.x
        && b.lo.x < a.hi.x
        && a.lo.y < b.hi.y
        && b.lo.y < a.hi.y
        && a.lo.z < b.hi.z
        && b.lo.z < a.hi.z
}

/// Pickups and hits count a touch, faces included.
pub const fn touches(a: Box, b: Box) -> bool {
    a.lo.x <= b.hi.x
        && b.lo.x <= a.hi.x
        && a.lo.y <= b.hi.y
        && b.lo.y <= a.hi.y
        && a.lo.z <= b.hi.z
        && b.lo.z <= a.hi.z
}

/// The footprints meet strictly.
pub const fn over(a: Box, b: Box) -> bool {
    a.lo.x < b.hi.x && b.lo.x < a.hi.x && a.lo.z < b.hi.z && b.lo.z < a.hi.z
}

/// 0 up to `n`, then down to 0, period `2n`. `mod` is floored (contract section 1): `rem_euclid` is floored for a positive divisor.
pub const fn triangle(t: i32, n: i32) -> i32 {
    let p = t.rem_euclid(2 * n);
    if p <= n { p } else { 2 * n - p }
}

pub fn approach(v: i32, target: i32, step: i32) -> i32 {
    if v < target {
        (v + step).min(target)
    } else {
        (v - step).max(target)
    }
}

pub fn vec_text(v: Vec3) -> String {
    v.to_string()
}
