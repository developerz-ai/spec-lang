// Projected by hand from the @cases of q, and level (src/hop/levels.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "harness.hpp"
#include "hop/levels/levels.hpp"

using namespace hop::levels;

static_assert(grid::q(4) == 256 && grid::q(-2) == -128);

TEST(q_cases) {
  CHECK_EQ(grid::q(4), 256);
  CHECK_EQ(grid::q(-2), -128);
}

TEST(level_picks_by_number) {
  CHECK(level(first_level) == meadow);
  CHECK(level(second_level) == ridge);
  CHECK_EQ(level(first_level).name, "meadow");
  CHECK_EQ(level(second_level).name, "ridge");
}

TEST(a_level_number_is_1_or_2) {
  CHECK(!LevelNo::make(0));
  CHECK(LevelNo::make(1).has_value());
  CHECK(LevelNo::make(2).has_value());
  CHECK(!LevelNo::make(3));
}
