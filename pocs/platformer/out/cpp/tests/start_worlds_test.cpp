// Projected by hand from the tests past_the_first_ledge, running_at_the_step, under_a_low_ceiling, riding_the_mover, above_the_mover, at_the_third_gem, falling_on_the_crawler, running_at_the_crawler, about_to_fall_out, about_to_fall_out_for_the_last_time, beside_the_flag, about_to_fall_out_holding_a_gem, rising_beside_the_crawler, under_the_mover, falling_fast_onto_the_floor (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "harness.hpp"
#include "hop/physics/bodies.hpp"
#include "support/show.hpp"
#include "support/worlds.hpp"

using hop::geom::Vec3;
using hop::model::Mode;
using hop::world::World;

namespace {

// Each of these spec tests has an empty body: it passes when its @given world can be built. The
// states it does not name keep their declared values, and the body stands outside every solid.
void check_start_world(const World& w) {
  CHECK_EQ(w.ticks, 0);
  CHECK(w.mode == Mode::Playing);
  for (const hop::model::Solid& s : w.stage.solids) {
    CHECK(!hop::geom::overlaps(hop::physics::body(w.player.at), s.box));
  }
}

}  // namespace

TEST(past_the_first_ledge) {
  const World w = worlds::past_the_first_ledge();
  check_start_world(w);
  CHECK_EQ(w.player.at, (Vec3{1090, 0, 0}));
  CHECK_EQ(w.player.coyote, 6);
  CHECK(w.player.grounded);
  CHECK_EQ(w.lives, 3);
}

TEST(running_at_the_step) {
  const World w = worlds::running_at_the_step();
  check_start_world(w);
  CHECK_EQ(w.player.vel, (Vec3{16, 0, 0}));
}

TEST(under_a_low_ceiling) {
  const World w = worlds::under_a_low_ceiling();
  check_start_world(w);
  CHECK_EQ(w.stage.name, "box");
  CHECK_EQ(w.stage.solids.size(), std::size_t{2});
  CHECK_EQ(w.stage.goal, (Vec3{100000, 0, 0}));
}

TEST(riding_the_mover) {
  const World w = worlds::riding_the_mover();
  check_start_world(w);
  CHECK(w.stage == hop::levels::ridge);
  CHECK(w.player.riding == std::optional<hop::geom::Int>(1));
}

TEST(above_the_mover) {
  const World w = worlds::above_the_mover();
  check_start_world(w);
  CHECK_EQ(w.player.at, (Vec3{1600, 200, 0}));
  CHECK(!w.player.grounded);
}

TEST(at_the_third_gem) {
  const World w = worlds::at_the_third_gem();
  check_start_world(w);
  CHECK_EQ(w.player.at.x, hop::levels::meadow.gems[2].at.x);
}

TEST(falling_on_the_crawler) {
  const World w = worlds::falling_on_the_crawler();
  check_start_world(w);
  CHECK(w.crawlers.contains(1));
  CHECK_EQ(w.player.vel, (Vec3{0, -20, 0}));
}

TEST(running_at_the_crawler) {
  const World w = worlds::running_at_the_crawler();
  check_start_world(w);
  CHECK_EQ(w.crawlers.size(), std::size_t{1});
  CHECK_EQ(w.crawlers.values()[0].at, (Vec3{3072, 256, 0}));
}

TEST(about_to_fall_out) {
  const World w = worlds::about_to_fall_out();
  check_start_world(w);
  CHECK_EQ(w.lives, 3);
  CHECK_EQ(w.player.at.y, -2040);
}

TEST(about_to_fall_out_for_the_last_time) {
  const World w = worlds::about_to_fall_out_for_the_last_time();
  check_start_world(w);
  CHECK_EQ(w.lives, 1);
}

TEST(beside_the_flag) {
  const World w = worlds::beside_the_flag();
  check_start_world(w);
  CHECK_EQ(w.player.at, (Vec3{5000, 0, 0}));
}

TEST(about_to_fall_out_holding_a_gem) {
  const World w = worlds::about_to_fall_out_holding_a_gem();
  check_start_world(w);
  CHECK(w.taken == hop::world::Taken{1});
  CHECK_EQ(w.lives, 3);
}

TEST(rising_beside_the_crawler) {
  const World w = worlds::rising_beside_the_crawler();
  check_start_world(w);
  CHECK_EQ(w.player.at, (Vec3{2920, 330, 0}));
  CHECK_EQ(w.player.vel, (Vec3{0, 20, 0}));
  CHECK(w.crawlers.contains(1));
}

TEST(under_the_mover) {
  const World w = worlds::under_the_mover();
  check_start_world(w);
  CHECK(w.stage == hop::levels::ridge);
  CHECK_EQ(w.player.at, (Vec3{1600, -200, 0}));
}

TEST(falling_fast_onto_the_floor) {
  const World w = worlds::falling_fast_onto_the_floor();
  check_start_world(w);
  CHECK_EQ(w.player.vel, (Vec3{0, -48, 0}));
}
