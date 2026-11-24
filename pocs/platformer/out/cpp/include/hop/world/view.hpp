// Projected by hand from View, view (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <span>

#include "hop/world/world.hpp"

namespace hop::world {

// Everything the screen needs, read once per frame. A fn writes nothing, so the view borrows the
// world instead of copying it; it must not outlive the world or a rule call.
struct View {
  const model::Level& stage;
  Int ticks;
  const model::Player& player;
  const Taken& taken;
  std::span<const model::Crawler> crawlers;
  Int lives;
  model::Mode mode;
};

inline View view(const World& w) {
  return {w.stage, w.ticks, w.player, w.taken, w.crawlers.values(), w.lives, w.mode};
}

}  // namespace hop::world
