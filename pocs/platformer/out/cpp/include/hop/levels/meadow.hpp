// Projected by hand from meadow (src/hop/levels.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <array>

#include "hop/levels/grid.hpp"

namespace hop::levels {

// The level is a pure constant, so it is computed in the build and lives in read-only data.
namespace meadow_data {
using namespace grid;
inline constexpr std::array<model::Solid, 4> solids{
    slab(at(-8, -4, -8), at(16, 0, 8), grass),
    slab(at(24, -4, -8), at(40, 0, 8), grass),
    slab(at(40, -4, -8), at(56, 4, 8), stone),
    slab(at(64, -4, -8), at(84, 0, 8), grass),
};
inline constexpr std::array<model::Gem, 3> gems{
    model::Gem{1, at(32, 6, 0)},
    model::Gem{2, at(60, 10, 0)},
    model::Gem{3, at(72, 2, 0)},
};
inline constexpr std::array<model::Crawler, 1> crawlers{
    model::Crawler{1, at(48, 4, 0), q(43), q(53), 1, 2},
};
}  // namespace meadow_data

// Teaches running, a gap, a step up, a crawler and the flag.
inline constexpr model::Level meadow{
    "meadow",          grid::at(0, 0, 0),     meadow_data::solids, {},
    meadow_data::gems, meadow_data::crawlers, grid::at(80, 0, 0),
};

}  // namespace hop::levels
