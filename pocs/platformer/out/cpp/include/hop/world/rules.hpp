// Projected by hand from rule start, rule step, the error Over, fn summary (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <expected>
#include <string>

#include "hop/model/pad.hpp"
#include "hop/world/lines.hpp"
#include "hop/world/world.hpp"

namespace hop::world {

// The one refusal: @guard(mode == Playing else Over).
enum class Refusal { Over };

// Every level starts the same way: three lives, nothing taken, every crawler home.
void start(World& world, levels::LevelNo level, Lines& out);

// One tick of play: carry, steer, jump, fall, then everything the hero touches.
// Refused with Over when the game is not Playing; the world is then unchanged and no line is sent.
std::expected<void, Refusal> step(World& world, const model::Pad& pad, Lines& out);

// One line that sums up a run, printed after the last tick.
std::string summary(const World& world);

}  // namespace hop::world
