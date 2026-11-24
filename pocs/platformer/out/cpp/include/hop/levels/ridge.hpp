// Projected by hand from ridge (src/hop/levels.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <array>

#include "hop/levels/grid.hpp"

namespace hop::levels {

namespace ridge_data {
using namespace grid;
inline constexpr std::array<model::Solid, 4> solids{
    slab(at(-8, -4, -8), at(16, 0, 8), grass),
    slab(at(52, -4, -8), at(60, 0, 8), grass),
    slab(at(60, -4, -8), at(76, 5, 8), stone),
    slab(at(80, -4, -8), at(100, 5, 8), grass),
};
inline constexpr std::array<model::Mover, 1> movers{
    model::Mover{1, {at(20, -2, -6), at(32, 0, 6)}, {8, 0, 0}, 128, wood},
};
inline constexpr std::array<model::Gem, 3> gems{
    model::Gem{1, at(34, 5, 0)},
    model::Gem{2, at(68, 9, 0)},
    model::Gem{3, at(90, 9, 0)},
};
inline constexpr std::array<model::Crawler, 2> crawlers{
    model::Crawler{1, at(68, 5, 0), q(62), q(74), 1, 3},
    model::Crawler{2, at(88, 5, 0), q(82), q(94), -1, 2},
};
}  // namespace ridge_data

// A moving platform over a wide gap, then two crawlers on high ground a jump can reach.
inline constexpr model::Level ridge{
    "ridge",          grid::at(0, 0, 0),    ridge_data::solids, ridge_data::movers,
    ridge_data::gems, ridge_data::crawlers, grid::at(96, 5, 0),
};

}  // namespace hop::levels
