// Projected by hand from rule lose (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "lose.hpp"

namespace hop::world {

void lose(World& world, std::string_view why, Lines& out) {
  out.event(world.ticks, why);
  world.lives -= 1;
  world.player = model::Player{world.stage.start};
  if (world.lives == 0) {
    world.mode = model::Mode::Lost;
    out.event(world.ticks, "lost");
  }
}

}  // namespace hop::world
