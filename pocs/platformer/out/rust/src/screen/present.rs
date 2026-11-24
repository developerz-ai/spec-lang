// Projected by hand from clear_background, begin_mode_3d, end_mode_3d, begin_lit, end_lit, draw_cube, draw_cube_wires, draw_model, draw_text, present (src/hop/screen.slg). Stand-in for spec-lang build output; change the spec, not this file.

use std::ffi::CString;

use super::assets::Assets;
use super::convert::{color, cube, vector};
use super::ffi;
use crate::render::{self, Draw};
use crate::world::World;

/// The frame as data goes to raylib in its own order: background, 3D pass under the lit shader, then the HUD.
/// Called between `BeginDrawing`/`EndDrawing` or `BeginTextureMode`/`EndTextureMode`, with `assets` loaded after the window opened.
pub fn present(world: &World, assets: &Assets) {
    let f = render::scene(world.view());
    // SAFETY: every argument is a plain struct by value or a pointer that outlives the call; raylib keeps none of them.
    unsafe {
        ffi::ClearBackground(color(f.sky));
        ffi::BeginMode3D(ffi::Camera3D {
            position: vector(f.eye),
            target: vector(f.target),
            up: ffi::Vector3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            fovy: 45.0,
            projection: ffi::CAMERA_PERSPECTIVE,
        });
        // `uTime` is the tick count as a float: exact below 2^24, and a script has at most 10^6 ticks.
        let time = f.time as f32;
        ffi::SetShaderValue(
            assets.lit,
            assets.time_loc,
            (&raw const time).cast(),
            ffi::SHADER_UNIFORM_FLOAT,
        );
        ffi::BeginShaderMode(assets.lit);
        for d in &f.world {
            match *d {
                Draw::Cube { r#box, color: c } => {
                    let (centre, w, h, d) = cube(r#box);
                    ffi::DrawCube(centre, w, h, d, color(c));
                }
                Draw::Wires { r#box, color: c } => {
                    let (centre, w, h, d) = cube(r#box);
                    ffi::DrawCubeWires(centre, w, h, d, color(c));
                }
                Draw::Mesh { asset, at, yaw } => ffi::DrawModelEx(
                    assets.model(asset),
                    vector(at),
                    ffi::Vector3 {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    },
                    yaw as f32,
                    ffi::Vector3 {
                        x: 1.0,
                        y: 1.0,
                        z: 1.0,
                    },
                    ffi::WHITE,
                ),
            }
        }
        ffi::EndShaderMode();
        ffi::EndMode3D();
        for l in &f.labels {
            // A label is ASCII built from words and integers: it has no NUL byte.
            if let Ok(text) = CString::new(l.text.as_str()) {
                ffi::DrawText(text.as_ptr(), l.x, l.y, l.size, color(l.color));
            }
        }
    }
}
