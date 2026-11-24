// Projected by hand from the @cases of pad_of and the tests of hop.tests.runs (src/hop/tests/runs.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "harness.hpp"
#include "support/play.hpp"
#include "support/scripts.hpp"

using hop::model::Mode;
using hop::model::Pad;
using hop::world::World;

TEST(pad_of_cases) {
  CHECK(scripts::pad_of("-") == Pad{});
  CHECK(scripts::pad_of("RJ") == (Pad{.right = true, .jump = true}));
  CHECK(scripts::pad_of("LFB") == (Pad{.left = true, .forward = true, .back = true}));
}

// The meadow is winnable, with every gem, without losing a life.
TEST(the_meadow_can_be_won) {
  World world;
  play::start(world, 1);
  for (const scripts::Hold& h : scripts::meadow_win) {
    for (int i = 1; i <= h.ticks; ++i) {
      play::step(world, scripts::pad_of(h.keys));
    }
  }
  CHECK(world.mode == Mode::Won);
  CHECK_EQ(world.ticks, 316);
  CHECK_EQ(world.taken.size(), std::size_t{3});
  CHECK_EQ(world.lives, 3);
}

// The ridge is winnable too.
TEST(the_ridge_can_be_won) {
  World world;
  play::start(world, 2);
  for (const scripts::Hold& h : scripts::ridge_win) {
    for (int i = 1; i <= h.ticks; ++i) {
      if (world.mode == Mode::Playing) {
        play::step(world, scripts::pad_of(h.keys));
      }
    }
  }
  CHECK(world.mode == Mode::Won);
  CHECK_EQ(world.ticks, 549);
  CHECK_EQ(world.taken.size(), std::size_t{3});
  CHECK_EQ(world.lives, 3);
}
