// Projected by hand from the tests of hop.tests.touches (src/hop/tests/touches.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "harness.hpp"
#include "support/play.hpp"
#include "support/show.hpp"
#include "support/worlds.hpp"

using hop::model::Mode;
using hop::world::Taken;
using hop::world::World;
using play::rest;
using play::right;

TEST(a_gem_is_taken_once) {
  World world = worlds::at_the_third_gem();
  hop::world::Lines out = hop::world::Lines::kept();
  play::step(world, rest, out);
  CHECK(world.taken == Taken{3});
  play::step(world, rest, out);
  CHECK(world.taken == Taken{3});
  CHECK_EQ(out.text(), "1 gem 3\n");
}

// A life lost keeps the gems taken.
TEST(a_fall_keeps_the_gems) {
  World world = worlds::about_to_fall_out_holding_a_gem();
  play::step(world, rest);
  CHECK_EQ(world.lives, 2);
  CHECK(world.taken == Taken{1});
}

// Only a falling body stomps; rising into a crawler hurts.
TEST(rising_into_a_crawler_hurts) {
  World world = worlds::rising_beside_the_crawler();
  play::step(world, rest);
  CHECK_EQ(world.lives, 2);
  CHECK(world.crawlers.contains(1));
}

// Falling onto the upper half of a crawler removes it and bounces.
TEST(a_stomp_removes_the_crawler) {
  World world = worlds::falling_on_the_crawler();
  play::step(world, rest);
  CHECK(!world.crawlers.contains(1));
  CHECK_EQ(world.player.vel.y, 24);
  CHECK_EQ(world.lives, 3);
}

// Walking into a crawler costs a life and restarts the hero.
TEST(a_crawler_hurts_from_the_side) {
  World world = worlds::running_at_the_crawler();
  play::step(world, right);
  CHECK_EQ(world.lives, 2);
  CHECK_EQ(world.player.at, hop::levels::meadow.start);
  CHECK(world.crawlers.contains(1));
}

TEST(falling_out_costs_a_life) {
  World world = worlds::about_to_fall_out();
  play::step(world, rest);
  CHECK_EQ(world.lives, 2);
  CHECK(world.mode == Mode::Playing);
}

TEST(the_last_life_ends_the_game) {
  World world = worlds::about_to_fall_out_for_the_last_time();
  play::step(world, rest);
  CHECK_EQ(world.lives, 0);
  CHECK(world.mode == Mode::Lost);
  play::step_fails_over(world, rest);
}

TEST(reaching_the_flag_wins) {
  World world = worlds::beside_the_flag();
  play::step(world, rest);
  CHECK(world.mode == Mode::Won);
  play::step_fails_over(world, rest);
}

TEST(starting_again_resets_everything) {
  World world;
  play::start(world, 1);
  for (int i = 1; i <= 5; ++i) {
    play::step(world, right);
  }
  play::start(world, 2);
  CHECK(world.taken == Taken{});
  CHECK_EQ(world.ticks, 0);
  CHECK_EQ(world.crawlers.size(), std::size_t{2});
  CHECK_EQ(world.player.at, hop::levels::ridge.start);
}

// The lines a rule sends are part of what it does: the order within one tick.
TEST(the_last_fall_says_fall_then_lost) {
  World world = worlds::about_to_fall_out_for_the_last_time();
  hop::world::Lines lines = hop::world::Lines::kept();
  CHECK(hop::world::step(world, rest, lines).has_value());
  CHECK_EQ(lines.text(), "1 fall\n1 lost\n");
  CHECK_EQ(hop::world::summary(world), "end 1 lost gems 0/3 lives 0 at 0,0,0");
}

// draw and shot hand the rules a dropping sink: the same world, no text.
TEST(a_dropping_sink_changes_nothing_but_the_text) {
  World loud = worlds::at_the_third_gem();
  World quiet = loud;
  hop::world::Lines kept = hop::world::Lines::kept();
  hop::world::Lines dropped = hop::world::Lines::dropped();
  CHECK(hop::world::step(loud, rest, kept).has_value());
  CHECK(hop::world::step(quiet, rest, dropped).has_value());
  CHECK(loud == quiet);
  CHECK_EQ(kept.text(), "1 gem 3\n");
  CHECK_EQ(dropped.text(), "");
}
