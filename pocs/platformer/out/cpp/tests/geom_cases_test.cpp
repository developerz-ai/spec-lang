// Projected by hand from the @cases of plus, around_feet, overlaps, touches, triangle, approach, vec_text (src/hop/geom.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "harness.hpp"
#include "hop/geom/box.hpp"
#include "hop/geom/scalar.hpp"
#include "hop/geom/vec3.hpp"
#include "support/show.hpp"

using namespace hop::geom;

// Pure with constant inputs is computed in the build: the same tables, checked by the compiler.
static_assert(plus({1, 2, 3}, {10, 20, 30}) == Vec3{11, 22, 33});
static_assert(triangle(5, 4) == 3 && triangle(8, 4) == 0);
static_assert(approach(-5, 0, 1) == -4);

TEST(plus_cases) { CHECK_EQ(plus({1, 2, 3}, {10, 20, 30}), (Vec3{11, 22, 33})); }

TEST(around_feet_cases) {
  CHECK(around_feet({0, 0, 0}, {64, 192, 64}) == (Box{{-64, 0, -64}, {64, 192, 64}}));
}

TEST(overlaps_cases) {
  CHECK_EQ(overlaps({{0, 0, 0}, {2, 2, 2}}, {{1, 1, 1}, {3, 3, 3}}), true);
  CHECK_EQ(overlaps({{0, 0, 0}, {2, 2, 2}}, {{2, 0, 0}, {4, 2, 2}}), false);
}

TEST(touches_cases) {
  CHECK_EQ(touches({{0, 0, 0}, {2, 2, 2}}, {{2, 0, 0}, {4, 2, 2}}), true);
  CHECK_EQ(touches({{0, 0, 0}, {2, 2, 2}}, {{3, 0, 0}, {4, 2, 2}}), false);
}

TEST(triangle_cases) {
  CHECK_EQ(triangle(0, 4), 0);
  CHECK_EQ(triangle(3, 4), 3);
  CHECK_EQ(triangle(4, 4), 4);
  CHECK_EQ(triangle(5, 4), 3);
  CHECK_EQ(triangle(8, 4), 0);
  CHECK_EQ(triangle(9, 4), 1);
}

TEST(approach_cases) {
  CHECK_EQ(approach(0, 3, 2), 2);
  CHECK_EQ(approach(5, 3, 2), 3);
  CHECK_EQ(approach(5, 0, 2), 3);
  CHECK_EQ(approach(-5, 0, 1), -4);
}

TEST(vec_text_cases) { CHECK_EQ(vec_text({1, -2, 3}), "1,-2,3"); }

// Contract section 1: mod is floored, in 0..<n for n > 0.
TEST(mod_is_floored) {
  CHECK_EQ(floor_mod(-1, 8), 7);
  CHECK_EQ(floor_mod(-8, 8), 0);
  CHECK_EQ(floor_mod(9, 8), 1);
}
