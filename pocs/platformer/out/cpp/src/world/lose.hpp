// Projected by hand from rule lose (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <string_view>

#include "hop/world/lines.hpp"
#include "hop/world/world.hpp"

namespace hop::world {

// A hit or a fall costs a life and sends the hero back to the start; the last one ends the game.
// Private to hop.world and called only inside step, which it joins. Its guard (mode == Playing
// else Over) is not tested again: step's own guard held, step calls lose at most once (`alive`),
// and nothing before the call writes mode, so the guard cannot fail there.
void lose(World& world, std::string_view why, Lines& out);

}  // namespace hop::world
