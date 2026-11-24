// Projected by hand from body, crawler_box, mover_box, mover_shift (src/hop/physics.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include "hop/geom/box.hpp"
#include "hop/geom/scalar.hpp"
#include "hop/model/level.hpp"
#include "hop/physics/constants.hpp"

namespace hop::physics {

constexpr geom::Box body(geom::Vec3 at) { return geom::around_feet(at, body_half); }

constexpr geom::Box crawler_box(const model::Crawler& c) { return geom::around_feet(c.at, crawler_half); }

constexpr geom::Box mover_box(const model::Mover& m, Int t) {
  return geom::shifted(m.home, geom::scaled(m.step, geom::triangle(t, m.span)));
}

// A rider moves exactly as far as its mover did this tick.
// The spec subtracts two boxes' lo corners; Int is exact, so home cancels and only the two
// triangle values are needed: (home.lo + step*a) - (home.lo + step*b) = step*(a - b).
constexpr geom::Vec3 mover_shift(const model::Mover& m, Int t) {
  return geom::scaled(m.step, geom::triangle(t, m.span) - geom::triangle(t - 1, m.span));
}

}  // namespace hop::physics
