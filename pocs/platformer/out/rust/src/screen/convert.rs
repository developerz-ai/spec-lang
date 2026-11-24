// Projected by hand from the edge conversions of contract section 5 for Sub, Vec3, Box and Rgb (src/hop/geom.slg, src/hop/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

use super::ffi::{BoundingBox, Color, Vector3};
use crate::geom::{Box, Sub, Vec3};
use crate::model::Rgb;

/// A `Sub` as `(float)v / 256.0f`: exact, since `|v| <= 2^20` fits the 24-bit mantissa and 256 is a power of two.
pub fn metres(v: Sub) -> f32 {
    v as f32 / 256.0
}

pub fn vector(v: Vec3) -> Vector3 {
    Vector3 {
        x: metres(v.x),
        y: metres(v.y),
        z: metres(v.z),
    }
}

pub fn color(c: Rgb) -> Color {
    Color {
        r: c.r,
        g: c.g,
        b: c.b,
        a: 255,
    }
}

pub fn bounding_box(b: Box) -> BoundingBox {
    BoundingBox {
        min: vector(b.lo),
        max: vector(b.hi),
    }
}

/// `DrawCube`'s arguments: `centre = (lo + hi) / 512.0f` per axis, then width, height, depth `(hi - lo) / 256.0f`.
pub fn cube(b: Box) -> (Vector3, f32, f32, f32) {
    let centre = Vector3 {
        x: (b.lo.x + b.hi.x) as f32 / 512.0,
        y: (b.lo.y + b.hi.y) as f32 / 512.0,
        z: (b.lo.z + b.hi.z) as f32 / 512.0,
    };
    (
        centre,
        metres(b.hi.x - b.lo.x),
        metres(b.hi.y - b.lo.y),
        metres(b.hi.z - b.lo.z),
    )
}
