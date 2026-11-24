// Projected by hand from eye_offset, look_height, bob, scene (src/hop/render.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include "hop/geom/scalar.hpp"
#include "hop/render/frame.hpp"
#include "hop/world/view.hpp"

namespace hop::render {

// The camera looks at the hero from the side and a little above, at a fixed distance.
inline constexpr geom::Vec3 eye_offset{0, 512, 2048};
inline constexpr Int look_height = 192;

// A gem bobs a quarter of a metre and turns three degrees a tick.
constexpr Int bob(Int t) { return geom::triangle(t, 32) * 2 - 32; }

// Solids, then movers, gems, crawlers, the flag and last the hero.
Frame scene(const world::View& v);

}  // namespace hop::render
