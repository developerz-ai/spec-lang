// Projected by hand from every foreign c "raylib#..." declaration (src/hop/screen.slg) and the calls of contract sections 2, 4 and 5. Stand-in for spec-lang build output; change the spec, not this file.
//! raylib 5.5, declared by hand against `vendor/raylib/include/raylib.h`. Layouts are `#[repr(C)]` copies of the C structs;
//! the library is linked at build time (`build.rs`). Only the fields this program reads are named in full; the rest keep
//! their C types so every size and offset matches.

use std::ffi::{c_char, c_float, c_int, c_uint, c_void};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vector3 {
    pub x: c_float,
    pub y: c_float,
    pub z: c_float,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Matrix {
    pub m: [c_float; 16],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct BoundingBox {
    pub min: Vector3,
    pub max: Vector3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Camera3D {
    pub position: Vector3,
    pub target: Vector3,
    pub up: Vector3,
    pub fovy: c_float,
    pub projection: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Shader {
    pub id: c_uint,
    pub locs: *mut c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Image {
    pub data: *mut c_void,
    pub width: c_int,
    pub height: c_int,
    pub mipmaps: c_int,
    pub format: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct Texture {
    pub id: c_uint,
    pub width: c_int,
    pub height: c_int,
    pub mipmaps: c_int,
    pub format: c_int,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct RenderTexture {
    pub id: c_uint,
    pub texture: Texture,
    pub depth: Texture,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub shader: Shader,
    pub maps: *mut c_void,
    pub params: [c_float; 4],
}

/// `Mesh`, `BoneInfo` and `Transform` are only pointed to; this program never reads them.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Model {
    pub transform: Matrix,
    pub mesh_count: c_int,
    pub material_count: c_int,
    pub meshes: *mut c_void,
    pub materials: *mut Material,
    pub mesh_material: *mut c_int,
    pub bone_count: c_int,
    pub bones: *mut c_void,
    pub bind_pose: *mut c_void,
}

pub const LOG_NONE: c_int = 7;
pub const FLAG_MSAA_4X_HINT: c_uint = 0x0000_0020;
pub const FLAG_WINDOW_HIDDEN: c_uint = 0x0000_0080;
pub const SHADER_UNIFORM_FLOAT: c_int = 0;
pub const CAMERA_PERSPECTIVE: c_int = 0;
pub const PIXELFORMAT_UNCOMPRESSED_R8G8B8A8: c_int = 7;
pub const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};

unsafe extern "C" {
    pub fn SetTraceLogLevel(level: c_int);
    pub fn SetConfigFlags(flags: c_uint);
    pub fn InitWindow(width: c_int, height: c_int, title: *const c_char);
    pub fn IsWindowReady() -> bool;
    pub fn WindowShouldClose() -> bool;
    pub fn CloseWindow();
    pub fn SetTargetFPS(fps: c_int);
    pub fn BeginDrawing();
    pub fn EndDrawing();

    pub fn ClearBackground(color: Color);
    pub fn BeginMode3D(camera: Camera3D);
    pub fn EndMode3D();
    pub fn BeginShaderMode(shader: Shader);
    pub fn EndShaderMode();
    pub fn BeginTextureMode(target: RenderTexture);
    pub fn EndTextureMode();

    pub fn DrawCube(
        position: Vector3,
        width: c_float,
        height: c_float,
        length: c_float,
        color: Color,
    );
    pub fn DrawCubeWires(
        position: Vector3,
        width: c_float,
        height: c_float,
        length: c_float,
        color: Color,
    );
    pub fn DrawModelEx(
        model: Model,
        position: Vector3,
        rotation_axis: Vector3,
        rotation_angle: c_float,
        scale: Vector3,
        tint: Color,
    );
    pub fn DrawText(text: *const c_char, x: c_int, y: c_int, size: c_int, color: Color);

    pub fn IsKeyDown(key: c_int) -> bool;
    pub fn CheckCollisionBoxes(a: BoundingBox, b: BoundingBox) -> bool;

    pub fn LoadShader(vs: *const c_char, fs: *const c_char) -> Shader;
    pub fn IsShaderValid(shader: Shader) -> bool;
    pub fn GetShaderLocation(shader: Shader, name: *const c_char) -> c_int;
    pub fn SetShaderValue(shader: Shader, loc: c_int, value: *const c_void, kind: c_int);
    pub fn UnloadShader(shader: Shader);
    pub fn rlGetShaderIdDefault() -> c_uint;

    pub fn LoadModel(file: *const c_char) -> Model;
    pub fn IsModelValid(model: Model) -> bool;
    pub fn UnloadModel(model: Model);

    pub fn LoadRenderTexture(width: c_int, height: c_int) -> RenderTexture;
    pub fn IsRenderTextureValid(target: RenderTexture) -> bool;
    pub fn UnloadRenderTexture(target: RenderTexture);
    pub fn LoadImageFromTexture(texture: Texture) -> Image;
    pub fn ImageFlipVertical(image: *mut Image);
    pub fn UnloadImage(image: Image);
    pub fn ExportImage(image: Image, file: *const c_char) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    /// Sizes from raylib.h on a 64-bit target: a wrong layout here is a wrong call at the edge.
    #[test]
    fn layouts_match_raylib_h() {
        assert_eq!(size_of::<Vector3>(), 12);
        assert_eq!(size_of::<Color>(), 4);
        assert_eq!(size_of::<BoundingBox>(), 24);
        assert_eq!(size_of::<Camera3D>(), 44);
        assert_eq!(size_of::<Shader>(), 16);
        assert_eq!(size_of::<Texture>(), 20);
        assert_eq!(size_of::<RenderTexture>(), 44);
        assert_eq!(size_of::<Image>(), 24);
        assert_eq!(size_of::<Material>(), 40);
        assert_eq!(size_of::<Matrix>(), 64);
        assert_eq!(size_of::<Model>(), 120);
        assert_eq!(align_of::<Model>(), 8);
    }
}
