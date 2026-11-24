// Projected by hand from q, at, slab, grass, stone, wood (src/hop/levels.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include "hop/model/level.hpp"

namespace hop::levels::grid {

using geom::Int;

inline constexpr model::Rgb grass{98, 170, 80};
inline constexpr model::Rgb stone{150, 150, 160};
inline constexpr model::Rgb wood{210, 150, 60};

// Levels are laid out on a quarter-metre grid: 64 sub-metres.
constexpr Int q(Int n) { return n * 64; }

constexpr geom::Vec3 at(Int x, Int y, Int z) { return {q(x), q(y), q(z)}; }

constexpr model::Solid slab(geom::Vec3 from, geom::Vec3 to, model::Rgb color) { return {{from, to}, color}; }

}  // namespace hop::levels::grid
