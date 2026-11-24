//! The `raylib` library's runtime for Rust: raylib 5.5, linked when the app is built (`link.rs`). `call` performs a
//! `foreign c "raylib#Name"` declaration of the program; `open` gives the window a game's runtime drives. Every call
//! into the C library is in this directory, and nowhere else in the app.

mod assets;
mod edge;
pub mod ffi;
mod window;

pub use window::{Picture, Spec, Window, open};

use crate::rt::data::Data;
use crate::rt::{R, foreign, stop};

/// The call needs a window and none is open (the program's checks): blocked, as the engine reports it.
fn no_window(symbol: &str) -> crate::rt::Halt {
    foreign(symbol, &format!("foreign c \"raylib#{symbol}\" with no window open"))
}

/// Performs `raylib#<symbol>`. A symbol this runtime has no adapter for stops, never guesses.
pub fn call(symbol: &str, args: &[Data]) -> R<Data> {
    let arg = |i: usize| edge::arg(args, i);
    // The one call that is plain arithmetic in raylib and needs no window.
    if symbol == "CheckCollisionBoxes" {
        let (a, b) = (edge::bounding_box(arg(0)?)?, edge::bounding_box(arg(1)?)?);
        // SAFETY: two plain structs by value; the function reads only them.
        return Ok(Data::Bool(unsafe { ffi::CheckCollisionBoxes(a, b) }));
    }
    window::with(|assets| {
        // SAFETY, for every call below: a window is open (`with`), every argument is a plain struct by value or a
        // pointer that outlives the call, and raylib keeps none of them.
        match symbol {
            "ClearBackground" => {
                let c = edge::color(arg(0)?)?;
                unsafe { ffi::ClearBackground(c) };
            }
            "BeginMode3D" => {
                let camera = edge::camera(arg(0)?, arg(1)?)?;
                unsafe { ffi::BeginMode3D(camera) };
            }
            "EndMode3D" => unsafe { ffi::EndMode3D() },
            "BeginShaderMode" => {
                let time = edge::float(arg(0)?)?;
                unsafe {
                    ffi::SetShaderValue(
                        assets.lit,
                        assets.time_loc,
                        (&raw const time).cast(),
                        ffi::SHADER_UNIFORM_FLOAT,
                    );
                    ffi::BeginShaderMode(assets.lit);
                }
            }
            "EndShaderMode" => unsafe { ffi::EndShaderMode() },
            "DrawCube" | "DrawCubeWires" => {
                let (centre, [w, h, d]) = edge::cube(arg(0)?)?;
                let c = edge::color(arg(1)?)?;
                let draw = if symbol == "DrawCube" { ffi::DrawCube } else { ffi::DrawCubeWires };
                unsafe { draw(centre, w, h, d, c) };
            }
            "DrawModelEx" => {
                let model = assets.model(arg(0)?).ok_or_else(|| stop("E-RUNTIME", "a model raylib has not loaded"))?;
                let (at, yaw) = (edge::vector(arg(1)?)?, edge::float(arg(2)?)?);
                let (up, scale) = (ffi::Vector3 { x: 0.0, y: 1.0, z: 0.0 }, ffi::Vector3 { x: 1.0, y: 1.0, z: 1.0 });
                unsafe { ffi::DrawModelEx(model, at, up, yaw, scale, ffi::WHITE) };
            }
            "DrawText" => {
                let t = edge::text(arg(0)?)?;
                let (x, y, size) = (edge::int(arg(1)?)?, edge::int(arg(2)?)?, edge::int(arg(3)?)?);
                let c = edge::color(arg(4)?)?;
                unsafe { ffi::DrawText(t.as_ptr(), x, y, size, c) };
            }
            "IsKeyDown" => {
                let key = edge::int(arg(0)?)?;
                return Ok(Data::Bool(unsafe { ffi::IsKeyDown(key) }));
            }
            _ => return Err(stop("E-UNSUPPORTED", format!("raylib#{symbol} has no adapter in libs/raylib/real/rust"))),
        }
        Ok(Data::Unit)
    })
    .unwrap_or_else(|| Err(no_window(symbol)))
}
