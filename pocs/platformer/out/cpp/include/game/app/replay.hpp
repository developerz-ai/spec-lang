// Projected by hand from the tick loop of run, draw and shot (docs/contract.md section 2) over rule step (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <cstdint>
#include <optional>

#include "game/script/script.hpp"
#include "hop/world/lines.hpp"
#include "hop/world/world.hpp"

namespace game::app {

// Steps the world through the script, one tick at a time, sending each tick's lines to out.
// Stops when the script ends, when the game is no longer Playing, or after tick `until`.
void replay(hop::world::World& world, const script::Script& script, std::optional<std::uint32_t> until,
            hop::world::Lines& out);

}  // namespace game::app
