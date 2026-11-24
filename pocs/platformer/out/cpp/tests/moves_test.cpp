// Projected by hand from the tests of hop.tests.moves (src/hop/tests/moves.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include <algorithm>

#include "harness.hpp"
#include "support/play.hpp"
#include "support/show.hpp"
#include "support/worlds.hpp"

using hop::geom::Int;
using hop::geom::Vec3;
using hop::world::World;
using play::jump;
using play::rest;
using play::right;

TEST(standing_still_stays_put) {
  World world;
  play::start(world, 1);
  for (int i = 1; i <= 30; ++i) {
    play::step(world, rest);
  }
  CHECK_EQ(world.player.at, (Vec3{0, 0, 0}));
  CHECK(world.player.grounded);
  CHECK_EQ(world.ticks, 30);
}

// The first tick starts in the air, so it gains 1; then 2 a tick up to 16.
TEST(running_reaches_full_speed) {
  World world;
  play::start(world, 1);
  for (int i = 1; i <= 10; ++i) {
    play::step(world, right);
  }
  CHECK_EQ(world.player.vel.x, 16);
  CHECK_EQ(world.player.at.x, 96);
  CHECK_EQ(world.player.yaw, 0);
}

// A held jump rises 38 + 36 + ... + 2 = 380 sub-metres.
TEST(a_held_jump_peaks_at_380) {
  World world;
  play::start(world, 1);
  play::step(world, rest);
  Int top = 0;
  for (int i = 1; i <= 50; ++i) {
    play::step(world, jump);
    top = std::max(top, world.player.at.y);
  }
  CHECK_EQ(top, 380);
  CHECK(world.player.grounded);
}

// Let go after one tick: 38, then cut to 16 and fall away: 14 + 12 + ... + 2 more.
TEST(a_short_hop_peaks_at_94) {
  World world;
  play::start(world, 1);
  play::step(world, rest);
  play::step(world, jump);
  Int top = 0;
  for (int i = 1; i <= 30; ++i) {
    play::step(world, rest);
    top = std::max(top, world.player.at.y);
  }
  CHECK_EQ(top, 94);
}

// Holding jump through a landing does not jump again.
TEST(a_jump_needs_a_fresh_press) {
  World world;
  play::start(world, 1);
  play::step(world, rest);
  for (int i = 1; i <= 60; ++i) {
    play::step(world, jump);
  }
  CHECK(world.player.grounded);
  CHECK_EQ(world.player.at.y, 0);
}

TEST(a_jump_just_after_the_edge_still_counts) {
  World world = worlds::past_the_first_ledge();
  play::step(world, rest);
  CHECK(!world.player.grounded);
  play::step(world, jump);
  CHECK_EQ(world.player.vel.y, 38);
}

TEST(a_jump_long_after_the_edge_does_not) {
  World world = worlds::past_the_first_ledge();
  for (int i = 1; i <= 7; ++i) {
    play::step(world, rest);
  }
  play::step(world, jump);
  CHECK(world.player.vel.y < 0);
}

TEST(a_wall_stops_the_body_flush) {
  World world = worlds::running_at_the_step();
  for (int i = 1; i <= 10; ++i) {
    play::step(world, right);
  }
  CHECK_EQ(world.player.at.x, 2496);
  CHECK_EQ(world.player.vel.x, 0);
}

TEST(a_ceiling_stops_a_jump) {
  World world = worlds::under_a_low_ceiling();
  Int top = 0;
  for (int i = 1; i <= 40; ++i) {
    play::step(world, jump);
    top = std::max(top, world.player.at.y);
  }
  CHECK_EQ(top, 208);
}

TEST(a_rider_moves_with_the_mover) {
  World world = worlds::riding_the_mover();
  for (int i = 1; i <= 10; ++i) {
    play::step(world, rest);
  }
  CHECK_EQ(world.player.at, (Vec3{1680, 0, 0}));
  CHECK(world.player.riding == std::optional<Int>(1));
}

TEST(a_mover_catches_a_falling_body) {
  World world = worlds::above_the_mover();
  for (int i = 1; i <= 40; ++i) {
    play::step(world, rest);
  }
  CHECK(world.player.riding == std::optional<Int>(1));
  CHECK_EQ(world.player.at.y, 0);
}

// A mover is one-way: a body below it keeps falling.
TEST(a_mover_never_catches_from_below) {
  World world = worlds::under_the_mover();
  play::step(world, rest);
  CHECK(world.player.at.y < -200);
  CHECK(!world.player.riding.has_value());
}

// The fastest fall still stops on a floor.
TEST(a_fast_fall_lands) {
  World world = worlds::falling_fast_onto_the_floor();
  for (int i = 1; i <= 12; ++i) {
    play::step(world, rest);
  }
  CHECK_EQ(world.player.at.y, 0);
  CHECK(world.player.grounded);
}
