// Projected by hand from the @cases of walk_target, facing, patrol (src/hop/physics.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "harness.hpp"
#include "hop/physics/bodies.hpp"
#include "hop/physics/crawl.hpp"
#include "hop/physics/steer.hpp"
#include "support/show.hpp"

using namespace hop::physics;
using hop::geom::Vec3;
using hop::model::Crawler;
using hop::model::Pad;

static_assert(walk_target({.left = true, .forward = true}) == Vec3{-11, 0, -11});
static_assert(facing({.left = true, .back = true}, 0) == 225);

TEST(walk_target_cases) {
  CHECK_EQ(walk_target(Pad{}), (Vec3{0, 0, 0}));
  CHECK_EQ(walk_target({.right = true}), (Vec3{16, 0, 0}));
  CHECK_EQ(walk_target({.left = true, .forward = true}), (Vec3{-11, 0, -11}));
  CHECK_EQ(walk_target({.left = true, .right = true}), (Vec3{0, 0, 0}));
}

TEST(facing_cases) {
  CHECK_EQ(facing(Pad{}, 90), 90);
  CHECK_EQ(facing({.right = true}, 90), 0);
  CHECK_EQ(facing({.forward = true}, 0), 90);
  CHECK_EQ(facing({.left = true, .back = true}, 0), 225);
}

TEST(patrol_cases) {
  CHECK(patrol(Crawler{1, {0, 0, 0}, -10, 10, 1, 4}) == (Crawler{1, {4, 0, 0}, -10, 10, 1, 4}));
  CHECK(patrol(Crawler{1, {8, 0, 0}, -10, 10, 1, 4}) == (Crawler{1, {10, 0, 0}, -10, 10, -1, 4}));
}

// mover_shift is projected as step * (triangle(t) - triangle(t - 1)); the spec subtracts two boxes.
TEST(mover_shift_is_the_difference_of_two_boxes) {
  const hop::model::Mover m{1, {{1280, -128, -384}, {2048, 0, 384}}, {8, -3, 2}, 5, {0, 0, 0}};
  for (hop::geom::Int t = 1; t <= 40; ++t) {
    CHECK_EQ(mover_shift(m, t), hop::geom::minus(mover_box(m, t).lo, mover_box(m, t - 1).lo));
  }
}
