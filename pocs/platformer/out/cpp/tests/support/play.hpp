// Projected by hand from the scenario steps of hop.tests (src/hop/tests/moves.slg): `call;` is a step that must be accepted, `call -> fail Over;` a refusal. Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include "harness.hpp"
#include "hop/world/rules.hpp"

namespace play {

inline constexpr hop::model::Pad rest{};
inline constexpr hop::model::Pad right{.right = true};
inline constexpr hop::model::Pad jump{.jump = true};

// world.start(n);
inline void start(hop::world::World& world, hop::geom::Int level) {
  hop::world::Lines lines = hop::world::Lines::kept();
  hop::world::start(world, *hop::levels::LevelNo::make(level), lines);
}

// world.step(pad); a refusal fails the test.
inline void step(hop::world::World& world, const hop::model::Pad& pad,
                 std::source_location where = std::source_location::current()) {
  hop::world::Lines lines = hop::world::Lines::kept();
  if (!hop::world::step(world, pad, lines)) {
    harness::fail("step was refused: Over", where);
  }
}

// world.step(pad); with the test's out.text collected in `lines`.
inline void step(hop::world::World& world, const hop::model::Pad& pad, hop::world::Lines& lines,
                 std::source_location where = std::source_location::current()) {
  if (!hop::world::step(world, pad, lines)) {
    harness::fail("step was refused: Over", where);
  }
}

// world.step(pad) -> fail Over; the world must be unchanged and no line sent.
inline void step_fails_over(hop::world::World& world, const hop::model::Pad& pad,
                            std::source_location where = std::source_location::current()) {
  const hop::world::World before = world;
  hop::world::Lines lines = hop::world::Lines::kept();
  const auto reply = hop::world::step(world, pad, lines);
  if (reply || reply.error() != hop::world::Refusal::Over) {
    harness::fail("step was accepted, expected fail Over", where);
  }
  if (!(world == before) || !lines.text().empty()) {
    harness::fail("a refused step changed the world or sent a line", where);
  }
}

}  // namespace play
