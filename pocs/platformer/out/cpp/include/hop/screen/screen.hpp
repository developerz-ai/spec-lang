// Projected by hand from check_collision_boxes, read_pad, present (src/hop/screen.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include "hop/geom/box.hpp"
#include "hop/model/pad.hpp"
#include "hop/world/world.hpp"

// The raylib edge as the rest of the program sees it: no raylib type crosses this header.
namespace hop::screen {

// raylib's own box test (CheckCollisionBoxes). Needs no window.
bool check_collision_boxes(const geom::Box& a, const geom::Box& b);

// Arrows or WASD to run, space to jump. Needs an open Session.
model::Pad read_pad();

// The frame as data goes to raylib in its own order: background, 3D pass under the lit shader,
// then the HUD. Needs an open Session.
void present(const world::World& world);

}  // namespace hop::screen
