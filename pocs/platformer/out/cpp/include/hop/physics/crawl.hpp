// Projected by hand from patrol, stomps (src/hop/physics.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include "hop/model/level.hpp"
#include "hop/physics/constants.hpp"

namespace hop::physics {

// A crawler walks to the end of its beat and turns.
constexpr model::Crawler patrol(model::Crawler c) {
  const Int x = c.at.x + c.dir * c.speed;
  if (x >= c.hi) {
    c.at.x = c.hi;
    c.dir = -1;
  } else if (x <= c.lo) {
    c.at.x = c.lo;
    c.dir = 1;
  } else {
    c.at.x = x;
  }
  return c;
}

// From above and falling, on the upper half: a stomp; any other touch hurts.
constexpr bool stomps(Int feet, Int vy, const model::Crawler& c) {
  return vy < 0 && feet >= c.at.y + stomp_line;
}

}  // namespace hop::physics
