// Projected by hand from Sub, unit, Vec3, plus, minus, scaled, vec_text (src/hop/geom.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <cstdint>
#include <string>

namespace hop::geom {

// Int: every value this program reaches fits in 32 bits (contract section 1), so no wider type is paid for.
using Int = std::int32_t;

// Sub = -1048576..=1048576: 4096 m either way, exact as a float at the C edge.
inline constexpr Int sub_min = -1048576;
inline constexpr Int sub_max = 1048576;

// 256 sub-metres to the metre.
inline constexpr Int unit = 256;

struct Vec3 {
  Int x;
  Int y;
  Int z;
  friend constexpr bool operator==(const Vec3&, const Vec3&) = default;
};

constexpr Vec3 plus(Vec3 a, Vec3 b) { return {a.x + b.x, a.y + b.y, a.z + b.z}; }
constexpr Vec3 minus(Vec3 a, Vec3 b) { return {a.x - b.x, a.y - b.y, a.z - b.z}; }
constexpr Vec3 scaled(Vec3 v, Int k) { return {v.x * k, v.y * k, v.z * k}; }

// "{v.x},{v.y},{v.z}"
std::string vec_text(Vec3 v);

}  // namespace hop::geom
