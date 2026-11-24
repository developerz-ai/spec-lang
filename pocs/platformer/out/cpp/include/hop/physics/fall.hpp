// Projected by hand from Fall, Placed, fall (src/hop/physics.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <algorithm>
#include <optional>
#include <ranges>
#include <span>

#include "hop/physics/bodies.hpp"

namespace hop::physics {

struct Fall {
  Int y;
  Int vel;
  bool grounded;
  std::optional<Int> riding;
  friend constexpr bool operator==(const Fall&, const Fall&) = default;
};

struct Placed {
  Int no;
  geom::Box box;
  friend constexpr bool operator==(const Placed&, const Placed&) = default;
};

// Solids stop the body both ways; a mover's top only catches feet that were above it.
// tops is any range of Placed: the tick rule passes a lazy view over the movers, so a tick that
// is stopped by a solid, or is not falling, never places a mover at all. Each filter/map/min/max
// chain of the spec is one pass.
template <std::ranges::input_range Tops>
  requires std::convertible_to<std::ranges::range_reference_t<Tops>, Placed>
constexpr Fall fall(geom::Vec3 at, Int v, std::span<const model::Solid> solids, Tops&& tops) {
  const geom::Box swept = body({at.x, at.y + v, at.z});
  bool hit = false;
  Int lowest_lo = geom::sub_max;
  Int highest_hi = geom::sub_min;
  for (const model::Solid& s : solids) {
    if (geom::overlaps(swept, s.box)) {
      hit = true;
      lowest_lo = std::min(lowest_lo, s.box.lo.y);
      highest_hi = std::max(highest_hi, s.box.hi.y);
    }
  }
  if (hit && v > 0) {
    return {lowest_lo - body_half.y, 0, false, std::nullopt};
  }
  if (hit) {
    return {highest_hi, 0, true, std::nullopt};
  }
  if (v < 0) {
    const geom::Box here = body(at);
    std::optional<Placed> best;  // the highest top; among equals, the lowest number
    for (const Placed& p : tops) {
      if (geom::over(here, p.box) && at.y >= p.box.hi.y && at.y + v < p.box.hi.y) {
        if (!best || p.box.hi.y > best->box.hi.y || (p.box.hi.y == best->box.hi.y && p.no < best->no)) {
          best = p;
        }
      }
    }
    if (best) {
      return {best->box.hi.y, 0, true, best->no};
    }
  }
  return {at.y + v, v, false, std::nullopt};
}

}  // namespace hop::physics
