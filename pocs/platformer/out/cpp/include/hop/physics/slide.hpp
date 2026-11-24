// Projected by hand from Axis, slide_x, slide_z (src/hop/physics.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <algorithm>
#include <span>

#include "hop/physics/bodies.hpp"

namespace hop::physics {

struct Axis {
  Int pos;
  Int vel;
  friend constexpr bool operator==(const Axis&, const Axis&) = default;
};

namespace detail {

// slide_x and slide_z are one body with the axis swapped. filter, map and min/max over a pure
// predicate fuse into one pass that keeps two integers and allocates nothing.
template <Int geom::Vec3::* along>
constexpr Axis slide(geom::Vec3 at, Int v, std::span<const model::Solid> solids) {
  if (v == 0) {
    return {at.*along, 0};
  }
  geom::Vec3 moved = at;
  moved.*along += v;
  const geom::Box swept = body(moved);
  bool hit = false;
  Int nearest_lo = geom::sub_max;
  Int farthest_hi = geom::sub_min;
  for (const model::Solid& s : solids) {
    if (geom::overlaps(swept, s.box)) {
      hit = true;
      nearest_lo = std::min(nearest_lo, s.box.lo.*along);
      farthest_hi = std::max(farthest_hi, s.box.hi.*along);
    }
  }
  if (!hit) {
    return {moved.*along, v};
  }
  return v > 0 ? Axis{nearest_lo - body_half.*along, 0} : Axis{farthest_hi + body_half.*along, 0};
}

}  // namespace detail

// Move on one axis, then stop flush against the first solid in the way.
constexpr Axis slide_x(geom::Vec3 at, Int v, std::span<const model::Solid> solids) {
  return detail::slide<&geom::Vec3::x>(at, v, solids);
}

constexpr Axis slide_z(geom::Vec3 at, Int v, std::span<const model::Solid> solids) {
  return detail::slide<&geom::Vec3::z>(at, v, solids);
}

}  // namespace hop::physics
