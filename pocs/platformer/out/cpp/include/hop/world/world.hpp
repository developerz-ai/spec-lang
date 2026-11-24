// Projected by hand from the seven states stage, ticks, player, taken, crawlers, lives, mode (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include "hop/levels/levels.hpp"
#include "hop/model/level.hpp"
#include "hop/model/player.hpp"
#include "hop/world/crawlers.hpp"
#include "hop/world/taken.hpp"

namespace hop::world {

using geom::Int;

// The one running level. The initial values are the ones the states declare; the program calls
// start before anything reads them, and a test's @given starts from them.
struct World {
  model::Level stage = levels::meadow;
  Int ticks = 0;
  model::Player player{levels::meadow.start};
  Taken taken;
  Crawlers crawlers;
  Int lives = 3;  // 0..=3
  model::Mode mode = model::Mode::Playing;
  friend bool operator==(const World&, const World&) = default;
};

}  // namespace hop::world
