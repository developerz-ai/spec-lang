// Projected by hand from the tick loop of run, draw and shot (docs/contract.md section 2) over rule step (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "game/app/replay.hpp"

#include "game/cli/command.hpp"
#include "hop/world/rules.hpp"

namespace game::app {

void replay(hop::world::World& world, const script::Script& script, std::optional<std::uint32_t> until,
            hop::world::Lines& out) {
  // Tick 0 is right after start; ticks never pass the script's million, so the ceiling is "no limit".
  const auto last = static_cast<hop::geom::Int>(until.value_or(cli::tick_ceiling));
  for (const script::Hold& hold : script.holds) {
    for (std::uint32_t i = 0; i < hold.ticks; ++i) {
      // A refused tick is the end of the run: step's guard is tested here, once, by the caller.
      if (world.ticks >= last || !hop::world::step(world, hold.pad, out)) {
        return;
      }
    }
  }
}

}  // namespace game::app
