// Projected by hand from rule start (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include <string>

#include "hop/world/rules.hpp"

namespace hop::world {

void start(World& world, levels::LevelNo level, Lines& out) {
  const model::Level& l = levels::level(level);
  world.stage = l;
  world.ticks = 0;
  world.player = model::Player{l.start};
  world.taken = Taken();
  world.crawlers.clear();
  for (const model::Crawler& c : l.crawlers) {
    world.crawlers.put(c);
  }
  world.lives = 3;
  world.mode = model::Mode::Playing;
  out.line(std::string("0 level ").append(l.name));
}

}  // namespace hop::world
