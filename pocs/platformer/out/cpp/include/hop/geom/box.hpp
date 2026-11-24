// Projected by hand from Box, shifted, around_feet, around_centre, overlaps, touches, over (src/hop/geom.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include "hop/geom/vec3.hpp"

namespace hop::geom {

// lo is the corner nearest minus infinity on every axis.
struct Box {
  Vec3 lo;
  Vec3 hi;
  friend constexpr bool operator==(const Box&, const Box&) = default;
};

constexpr Box shifted(const Box& b, Vec3 by) { return {plus(b.lo, by), plus(b.hi, by)}; }

// A body is placed by the centre of its base: its feet.
constexpr Box around_feet(Vec3 feet, Vec3 half) {
  return {{feet.x - half.x, feet.y, feet.z - half.z}, {feet.x + half.x, feet.y + half.y, feet.z + half.z}};
}

// A gem is placed by its centre.
constexpr Box around_centre(Vec3 c, Int half) {
  return {{c.x - half, c.y - half, c.z - half}, {c.x + half, c.y + half, c.z + half}};
}

// Solids block only when they share volume.
constexpr bool overlaps(const Box& a, const Box& b) {
  return a.lo.x < b.hi.x && b.lo.x < a.hi.x && a.lo.y < b.hi.y && b.lo.y < a.hi.y && a.lo.z < b.hi.z &&
         b.lo.z < a.hi.z;
}

// Pickups and hits count a touch, faces included: the test raylib's CheckCollisionBoxes makes.
constexpr bool touches(const Box& a, const Box& b) {
  return a.lo.x <= b.hi.x && b.lo.x <= a.hi.x && a.lo.y <= b.hi.y && b.lo.y <= a.hi.y && a.lo.z <= b.hi.z &&
         b.lo.z <= a.hi.z;
}

// The footprints meet strictly.
constexpr bool over(const Box& a, const Box& b) {
  return a.lo.x < b.hi.x && b.lo.x < a.hi.x && a.lo.z < b.hi.z && b.lo.z < a.hi.z;
}

}  // namespace hop::geom
