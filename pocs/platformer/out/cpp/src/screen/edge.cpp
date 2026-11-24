// Projected by hand from the foreign c "raylib#..." declarations of hop.screen, check_collision_boxes included (src/hop/screen.slg), with the conversions of docs/contract.md section 5. Stand-in for spec-lang build output; change the spec, not this file.
#include "edge.hpp"

#include "assets.hpp"
#include "hop/screen/screen.hpp"

namespace hop::screen {
namespace edge {
namespace {

struct CubeArgs {
  Vector3 centre;
  float width;
  float height;
  float depth;
};

// centre = (lo + hi) / 512 per axis; the sizes are (hi - lo) / 256.
CubeArgs cube_args(const geom::Box& b) {
  return {
      {static_cast<float>(b.lo.x + b.hi.x) / 512.0F, static_cast<float>(b.lo.y + b.hi.y) / 512.0F,
       static_cast<float>(b.lo.z + b.hi.z) / 512.0F},
      static_cast<float>(b.hi.x - b.lo.x) / 256.0F,
      static_cast<float>(b.hi.y - b.lo.y) / 256.0F,
      static_cast<float>(b.hi.z - b.lo.z) / 256.0F,
  };
}

}  // namespace

void clear_background(model::Rgb color) { ClearBackground(to_c(color)); }

// A perspective camera, y up, 45 degrees of vertical view.
void begin_mode_3d(geom::Vec3 eye, geom::Vec3 target) {
  BeginMode3D(Camera3D{to_c(eye), to_c(target), {0.0F, 1.0F, 0.0F}, 45.0F, CAMERA_PERSPECTIVE});
}

void end_mode_3d() { EndMode3D(); }

// The lit shader for the whole 3D pass; its one uniform is the tick count.
void begin_lit(Int time) {
  const Assets& loaded = assets();
  const float t = static_cast<float>(time);
  SetShaderValue(loaded.lit, loaded.time_loc, &t, SHADER_UNIFORM_FLOAT);
  BeginShaderMode(loaded.lit);
}

void end_lit() { EndShaderMode(); }

void draw_cube(const geom::Box& box, model::Rgb color) {
  const CubeArgs a = cube_args(box);
  DrawCube(a.centre, a.width, a.height, a.depth, to_c(color));
}

void draw_cube_wires(const geom::Box& box, model::Rgb color) {
  const CubeArgs a = cube_args(box);
  DrawCubeWires(a.centre, a.width, a.height, a.depth, to_c(color));
}

void draw_model(render::Asset asset, geom::Vec3 at, Int yaw) {
  DrawModelEx(assets().models[static_cast<std::size_t>(asset)], to_c(at), {0.0F, 1.0F, 0.0F},
              static_cast<float>(yaw), {1.0F, 1.0F, 1.0F}, Color{255, 255, 255, 255});
}

void draw_text(const std::string& text, Int x, Int y, Int size, model::Rgb color) {
  DrawText(text.c_str(), x, y, size, to_c(color));
}

bool key_down(Int key) { return IsKeyDown(key); }

}  // namespace edge

bool check_collision_boxes(const geom::Box& a, const geom::Box& b) {
  return CheckCollisionBoxes(edge::to_c(a), edge::to_c(b));
}

}  // namespace hop::screen
