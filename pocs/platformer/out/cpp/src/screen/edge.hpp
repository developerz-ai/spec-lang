// Projected by hand from the foreign c "raylib#..." declarations clear_background, begin_mode_3d, end_mode_3d, begin_lit, end_lit, draw_cube, draw_cube_wires, draw_model, draw_text, key_down (src/hop/screen.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <string>

#include "hop/geom/box.hpp"
#include "hop/model/rgb.hpp"
#include "hop/render/frame.hpp"
#include "raylib.h"

// Private to the screen module: the only headers that name raylib types live in src/screen.
namespace hop::screen::edge {

using geom::Int;

// A Sub becomes a float as (float)v / 256.0f, exact for every Sub (contract section 5).
constexpr float metres(Int v) { return static_cast<float>(v) / 256.0F; }
constexpr Vector3 to_c(geom::Vec3 v) { return {metres(v.x), metres(v.y), metres(v.z)}; }
constexpr Color to_c(model::Rgb c) { return {c.r, c.g, c.b, 255}; }
constexpr BoundingBox to_c(const geom::Box& b) { return {to_c(b.lo), to_c(b.hi)}; }

void clear_background(model::Rgb color);
void begin_mode_3d(geom::Vec3 eye, geom::Vec3 target);
void end_mode_3d();
void begin_lit(Int time);
void end_lit();
void draw_cube(const geom::Box& box, model::Rgb color);
void draw_cube_wires(const geom::Box& box, model::Rgb color);
void draw_model(render::Asset asset, geom::Vec3 at, Int yaw);
void draw_text(const std::string& text, Int x, Int y, Int size, model::Rgb color);
bool key_down(Int key);

}  // namespace hop::screen::edge
