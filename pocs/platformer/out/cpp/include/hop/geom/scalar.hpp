// Projected by hand from triangle, approach (src/hop/geom.slg) and floored mod (docs/contract.md section 1). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <algorithm>

#include "hop/geom/vec3.hpp"

namespace hop::geom {

// `t mod n` is floored: in 0..<n for n > 0.
constexpr Int floor_mod(Int t, Int n) {
  const Int r = t % n;
  return r < 0 ? r + n : r;
}

// Back and forth along a path without division: 0 up to n, then down to 0, period 2n.
constexpr Int triangle(Int t, Int n) {
  const Int p = floor_mod(t, 2 * n);
  return p <= n ? p : 2 * n - p;
}

constexpr Int approach(Int v, Int target, Int step) {
  return v < target ? std::min(v + step, target) : std::max(v - step, target);
}

}  // namespace hop::geom
