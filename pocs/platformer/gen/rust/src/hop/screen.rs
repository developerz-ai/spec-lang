//! Module `hop.screen`, printed by spec-lang from `src/hop/screen.slg`. Each item names its declaration and line; rebuild with `spec-lang build`, do not edit.

use crate::rt::{self, Int, R};
use crate::machine::Machine;

/// Constant `hop.screen.key_right`, `src/hop/screen.slg:48`.
pub const KEY_RIGHT: Int = 262;

/// Constant `hop.screen.key_left`, `src/hop/screen.slg:49`.
pub const KEY_LEFT: Int = 263;

/// Constant `hop.screen.key_down_arrow`, `src/hop/screen.slg:50`.
pub const KEY_DOWN_ARROW: Int = 264;

/// Constant `hop.screen.key_up`, `src/hop/screen.slg:51`.
pub const KEY_UP: Int = 265;

/// Constant `hop.screen.key_space`, `src/hop/screen.slg:52`.
pub const KEY_SPACE: Int = 32;

/// Constant `hop.screen.key_a`, `src/hop/screen.slg:53`.
pub const KEY_A: Int = 65;

/// Constant `hop.screen.key_d`, `src/hop/screen.slg:54`.
pub const KEY_D: Int = 68;

/// Constant `hop.screen.key_s`, `src/hop/screen.slg:55`.
pub const KEY_S: Int = 83;

/// Constant `hop.screen.key_w`, `src/hop/screen.slg:56`.
pub const KEY_W: Int = 87;

/// Fn `hop.screen.key_down`, `src/hop/screen.slg:42`.
pub fn key_down(key: Int) -> R<bool> {
    let reply = crate::libs::raylib::call(
        "IsKeyDown",
        &[rt::data::AsData::to_data(&key)],
    )?;
    <bool as rt::data::AsData>::from_data(&reply)
        .ok_or_else(|| rt::stop(
            "E-TYPE-MISMATCH",
            "`raylib#IsKeyDown` gave a value of another type",
        ))
}

/// raylib's own box test, so the game's rule can be checked against the library's
///
/// Fn `hop.screen.check_collision_boxes`, `src/hop/screen.slg:45`.
pub fn check_collision_boxes(
    a: crate::hop::geom::Box,
    b: crate::hop::geom::Box,
) -> R<bool> {
    rt::claim(
        crate::conform::r#box(&a)?,
        "argument `a` of hop.screen.check_collision_boxes",
    )?;
    rt::claim(
        crate::conform::r#box(&b)?,
        "argument `b` of hop.screen.check_collision_boxes",
    )?;
    let reply = crate::libs::raylib::call(
        "CheckCollisionBoxes",
        &[rt::data::AsData::to_data(&a), rt::data::AsData::to_data(&b)],
    )?;
    <bool as rt::data::AsData>::from_data(&reply)
        .ok_or_else(|| rt::stop(
            "E-TYPE-MISMATCH",
            "`raylib#CheckCollisionBoxes` gave a value of another type",
        ))
}

/// arrows or WASD to run, space to jump
///
/// Fn `hop.screen.read_pad`, `src/hop/screen.slg:58`.
pub fn read_pad() -> R<crate::hop::model::Pad> {
    crate::hop::model::Pad::new(
        Some(key_down(KEY_LEFT)? || key_down(KEY_A)?),
        Some(key_down(KEY_RIGHT)? || key_down(KEY_D)?),
        Some(key_down(KEY_UP)? || key_down(KEY_W)?),
        Some(key_down(KEY_DOWN_ARROW)? || key_down(KEY_S)?),
        Some(key_down(KEY_SPACE)?),
    )
}

/// Rule `hop.screen.clear_background`, `src/hop/screen.slg:13`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn clear_background(m: &mut Machine, color: crate::hop::model::Rgb) -> R<()> {
    m.atomic(|_m| {
        rt::claim(
            crate::conform::rgb(&color)?,
            "argument `color` of hop.screen.clear_background",
        )?;
        let reply = crate::libs::raylib::call(
            "ClearBackground",
            &[rt::data::AsData::to_data(&color)],
        )?;
        <() as rt::data::AsData>::from_data(&reply)
            .ok_or_else(|| rt::stop(
                "E-TYPE-MISMATCH",
                "`raylib#ClearBackground` gave a value of another type",
            ))
    })
}

/// a perspective camera, y up, 45 degrees of vertical view
///
/// Rule `hop.screen.begin_mode_3d`, `src/hop/screen.slg:16`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn begin_mode_3d(
    m: &mut Machine,
    eye: crate::hop::geom::Vec3,
    target: crate::hop::geom::Vec3,
) -> R<()> {
    m.atomic(|_m| {
        rt::claim(
            crate::conform::vec3(&eye)?,
            "argument `eye` of hop.screen.begin_mode_3d",
        )?;
        rt::claim(
            crate::conform::vec3(&target)?,
            "argument `target` of hop.screen.begin_mode_3d",
        )?;
        let reply = crate::libs::raylib::call(
            "BeginMode3D",
            &[rt::data::AsData::to_data(&eye), rt::data::AsData::to_data(&target)],
        )?;
        <() as rt::data::AsData>::from_data(&reply)
            .ok_or_else(|| rt::stop(
                "E-TYPE-MISMATCH",
                "`raylib#BeginMode3D` gave a value of another type",
            ))
    })
}

/// Rule `hop.screen.end_mode_3d`, `src/hop/screen.slg:20`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn end_mode_3d(m: &mut Machine) -> R<()> {
    m.atomic(|_m| {
        let reply = crate::libs::raylib::call("EndMode3D", &[])?;
        <() as rt::data::AsData>::from_data(&reply)
            .ok_or_else(|| rt::stop(
                "E-TYPE-MISMATCH",
                "`raylib#EndMode3D` gave a value of another type",
            ))
    })
}

/// the lit shader for the whole 3D pass; its one uniform is the tick count
///
/// Rule `hop.screen.begin_lit`, `src/hop/screen.slg:23`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn begin_lit(m: &mut Machine, time: Int) -> R<()> {
    m.atomic(|_m| {
        let reply = crate::libs::raylib::call(
            "BeginShaderMode",
            &[rt::data::AsData::to_data(&time)],
        )?;
        <() as rt::data::AsData>::from_data(&reply)
            .ok_or_else(|| rt::stop(
                "E-TYPE-MISMATCH",
                "`raylib#BeginShaderMode` gave a value of another type",
            ))
    })
}

/// Rule `hop.screen.end_lit`, `src/hop/screen.slg:27`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn end_lit(m: &mut Machine) -> R<()> {
    m.atomic(|_m| {
        let reply = crate::libs::raylib::call("EndShaderMode", &[])?;
        <() as rt::data::AsData>::from_data(&reply)
            .ok_or_else(|| rt::stop(
                "E-TYPE-MISMATCH",
                "`raylib#EndShaderMode` gave a value of another type",
            ))
    })
}

/// Rule `hop.screen.draw_cube`, `src/hop/screen.slg:30`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn draw_cube(
    m: &mut Machine,
    r#box: crate::hop::geom::Box,
    color: crate::hop::model::Rgb,
) -> R<()> {
    m.atomic(|_m| {
        rt::claim(
            crate::conform::r#box(&r#box)?,
            "argument `box` of hop.screen.draw_cube",
        )?;
        rt::claim(
            crate::conform::rgb(&color)?,
            "argument `color` of hop.screen.draw_cube",
        )?;
        let reply = crate::libs::raylib::call(
            "DrawCube",
            &[rt::data::AsData::to_data(&r#box), rt::data::AsData::to_data(&color)],
        )?;
        <() as rt::data::AsData>::from_data(&reply)
            .ok_or_else(|| rt::stop(
                "E-TYPE-MISMATCH",
                "`raylib#DrawCube` gave a value of another type",
            ))
    })
}

/// Rule `hop.screen.draw_cube_wires`, `src/hop/screen.slg:33`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn draw_cube_wires(
    m: &mut Machine,
    r#box: crate::hop::geom::Box,
    color: crate::hop::model::Rgb,
) -> R<()> {
    m.atomic(|_m| {
        rt::claim(
            crate::conform::r#box(&r#box)?,
            "argument `box` of hop.screen.draw_cube_wires",
        )?;
        rt::claim(
            crate::conform::rgb(&color)?,
            "argument `color` of hop.screen.draw_cube_wires",
        )?;
        let reply = crate::libs::raylib::call(
            "DrawCubeWires",
            &[rt::data::AsData::to_data(&r#box), rt::data::AsData::to_data(&color)],
        )?;
        <() as rt::data::AsData>::from_data(&reply)
            .ok_or_else(|| rt::stop(
                "E-TYPE-MISMATCH",
                "`raylib#DrawCubeWires` gave a value of another type",
            ))
    })
}

/// Rule `hop.screen.draw_model`, `src/hop/screen.slg:36`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn draw_model(
    m: &mut Machine,
    asset: crate::hop::render::Asset,
    at: crate::hop::geom::Vec3,
    yaw: Int,
) -> R<()> {
    m.atomic(|_m| {
        rt::claim(crate::conform::vec3(&at)?, "argument `at` of hop.screen.draw_model")?;
        rt::claim(
            crate::conform::range_0_359(&yaw)?,
            "argument `yaw` of hop.screen.draw_model",
        )?;
        let reply = crate::libs::raylib::call(
            "DrawModelEx",
            &[
                rt::data::AsData::to_data(&asset),
                rt::data::AsData::to_data(&at),
                rt::data::AsData::to_data(&yaw),
            ],
        )?;
        <() as rt::data::AsData>::from_data(&reply)
            .ok_or_else(|| rt::stop(
                "E-TYPE-MISMATCH",
                "`raylib#DrawModelEx` gave a value of another type",
            ))
    })
}

/// Rule `hop.screen.draw_text`, `src/hop/screen.slg:39`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn draw_text(
    m: &mut Machine,
    text: String,
    x: Int,
    y: Int,
    size: Int,
    color: crate::hop::model::Rgb,
) -> R<()> {
    m.atomic(|_m| {
        rt::claim(
            crate::conform::rgb(&color)?,
            "argument `color` of hop.screen.draw_text",
        )?;
        let reply = crate::libs::raylib::call(
            "DrawText",
            &[
                rt::data::AsData::to_data(&text),
                rt::data::AsData::to_data(&x),
                rt::data::AsData::to_data(&y),
                rt::data::AsData::to_data(&size),
                rt::data::AsData::to_data(&color),
            ],
        )?;
        <() as rt::data::AsData>::from_data(&reply)
            .ok_or_else(|| rt::stop(
                "E-TYPE-MISMATCH",
                "`raylib#DrawText` gave a value of another type",
            ))
    })
}

/// the frame as data goes to raylib in its own order: background, 3D pass under the lit shader, then the HUD
///
/// Rule `hop.screen.present`, `src/hop/screen.slg:68`.
///
/// One atomic step: on a refusal or any halt the world goes back to what it was before the call.
pub fn present(m: &mut Machine) -> R<()> {
    m.atomic(|m| {
        let f: crate::hop::render::Frame = crate::hop::render::scene(
            crate::hop::world::view(m)?,
        )?;
        clear_background(m, f.sky.clone())?;
        begin_mode_3d(m, f.eye.clone(), f.target.clone())?;
        begin_lit(m, f.time)?;
        for d in f.world.clone() {
            match &d {
                crate::hop::render::Draw::Cube { r#box: tmp1, color: tmp2, .. } => {
                    let b = (*tmp1).clone();
                    let color = (*tmp2).clone();
                    draw_cube(m, b.clone(), color.clone())?
                }
                crate::hop::render::Draw::Wires { r#box: tmp3, color: tmp4, .. } => {
                    let b = (*tmp3).clone();
                    let color = (*tmp4).clone();
                    draw_cube_wires(m, b.clone(), color.clone())?
                }
                crate::hop::render::Draw::Mesh {
                    asset: tmp5,
                    at: tmp6,
                    yaw: tmp7,
                    ..
                } => {
                    let asset = (*tmp5).clone();
                    let at = (*tmp6).clone();
                    let yaw = *tmp7;
                    draw_model(m, asset.clone(), at.clone(), yaw)?
                }
            };
        }
        end_lit(m)?;
        end_mode_3d(m)?;
        for l in f.labels.clone() {
            draw_text(m, l.text.clone(), l.x, l.y, l.size, l.color.clone())?;
        }
        Ok(())
    })
}
