//! The program's values as C values. The language has no `f32` and no C struct layout, so a length is a whole number of
//! 1/256 of a raylib unit and becomes `(float)v / 256.0f`; a vector is a record of three lengths, a colour a record of
//! three or four bytes (alpha 255 when left out), a box a record of two vectors. Records are read by position, so the
//! program's own names for them do not matter.

use super::ffi::{BoundingBox, CAMERA_PERSPECTIVE, Camera3D, Color, Vector3};
use crate::rt::data::Data;
use crate::rt::{Halt, Int, R, stop};
use std::ffi::{CString, c_int};

fn mismatch(want: &str) -> Halt {
    stop("E-TYPE-MISMATCH", format!("a raylib argument is not {want}"))
}

/// Argument `i` of a call.
pub fn arg(args: &[Data], i: usize) -> R<&Data> {
    args.get(i).ok_or_else(|| mismatch("given"))
}

fn fields(d: &Data) -> R<Vec<&Data>> {
    match d {
        Data::Record(fs) => Ok(fs.iter().map(|(_, v)| v).collect()),
        _ => Err(mismatch("a record")),
    }
}

fn whole(d: &Data) -> R<Int> {
    match d {
        Data::Int(n) => Ok(*n),
        _ => Err(mismatch("an Int")),
    }
}

/// A C `int`; a number it cannot hold stops.
pub fn int(d: &Data) -> R<c_int> {
    c_int::try_from(whole(d)?).map_err(|_| stop("E-RUNTIME", "an Int beyond a C int"))
}

/// A whole number as a C `float`, exact: beyond 2^24 it stops.
pub fn float(d: &Data) -> R<f32> {
    exact(whole(d)?)
}

fn exact(n: Int) -> R<f32> {
    if n.unsigned_abs() > 1 << 24 {
        return Err(stop("E-RUNTIME", "an Int a C float does not hold exactly"));
    }
    Ok(n as f32)
}

pub fn text(d: &Data) -> R<CString> {
    match d {
        Data::Text(t) => CString::new(t.as_str()).map_err(|_| stop("E-RUNTIME", "a Text with a NUL in a raylib call")),
        _ => Err(mismatch("Text")),
    }
}

fn lengths(d: &Data) -> R<[Int; 3]> {
    let [x, y, z] = fields(d)?[..] else { return Err(mismatch("a vector of three lengths")) };
    Ok([whole(x)?, whole(y)?, whole(z)?])
}

pub fn vector(d: &Data) -> R<Vector3> {
    let [x, y, z] = lengths(d)?;
    Ok(Vector3 { x: exact(x)? / 256.0, y: exact(y)? / 256.0, z: exact(z)? / 256.0 })
}

pub fn color(d: &Data) -> R<Color> {
    let parts = fields(d)?;
    let byte = |d: &Data| u8::try_from(whole(d)?).map_err(|_| mismatch("a colour of bytes"));
    match parts[..] {
        [r, g, b] => Ok(Color { r: byte(r)?, g: byte(g)?, b: byte(b)?, a: 255 }),
        [r, g, b, a] => Ok(Color { r: byte(r)?, g: byte(g)?, b: byte(b)?, a: byte(a)? }),
        _ => Err(mismatch("a colour of three or four bytes")),
    }
}

pub fn bounding_box(d: &Data) -> R<BoundingBox> {
    let [min, max] = fields(d)?[..] else { return Err(mismatch("a box of two vectors")) };
    Ok(BoundingBox { min: vector(min)?, max: vector(max)? })
}

/// `DrawCube`'s arguments: `centre = (lo + hi) / 512.0f` per axis, then width, height, depth `(hi - lo) / 256.0f`.
pub fn cube(d: &Data) -> R<(Vector3, [f32; 3])> {
    let [lo, hi] = fields(d)?[..] else { return Err(mismatch("a box of two vectors")) };
    let (lo, hi) = (lengths(lo)?, lengths(hi)?);
    let centre = |i: usize| Ok(exact(lo[i] + hi[i])? / 512.0);
    let size = |i: usize| Ok(exact(hi[i] - lo[i])? / 256.0);
    Ok((Vector3 { x: centre(0)?, y: centre(1)?, z: centre(2)? }, [size(0)?, size(1)?, size(2)?]))
}

/// A perspective camera, y up, 45 degrees of vertical view.
pub fn camera(position: &Data, target: &Data) -> R<Camera3D> {
    Ok(Camera3D {
        position: vector(position)?,
        target: vector(target)?,
        up: Vector3 { x: 0.0, y: 1.0, z: 0.0 },
        fovy: 45.0,
        projection: CAMERA_PERSPECTIVE,
    })
}
