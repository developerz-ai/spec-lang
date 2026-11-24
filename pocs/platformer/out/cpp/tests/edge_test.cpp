// Projected by hand from test touch_matches_raylib(a: Box, b: Box) (src/hop/tests/edge.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include <algorithm>
#include <cstdio>

#include "harness.hpp"
#include "hop/geom/box.hpp"
#include "hop/screen/screen.hpp"
#include "support/random.hpp"
#include "support/show.hpp"

using hop::geom::Box;
using hop::geom::Int;
using hop::geom::Vec3;

namespace {

constexpr int pair_count = 4000;

Int any_sub(support::Random& random) { return random.between(hop::geom::sub_min, hop::geom::sub_max); }
Vec3 any_vec(support::Random& random) { return {any_sub(random), any_sub(random), any_sub(random)}; }
Int clamped(Int v) { return std::clamp(v, hop::geom::sub_min, hop::geom::sub_max); }

// A Box is any two Vec3: nothing in the type says lo is below hi, so some are generated inside out.
Box any_box(support::Random& random) { return {any_vec(random), any_vec(random)}; }

// A box with lo below hi, small enough that two of them often meet.
Box ordered_box(support::Random& random) {
  const Vec3 lo = any_vec(random);
  return {lo,
          {clamped(lo.x + random.between(0, 4096)), clamped(lo.y + random.between(0, 4096)),
           clamped(lo.z + random.between(0, 4096))}};
}

// One side of b on one axis, placed a sub-metre short of, on, or past the facing side of a.
void place_axis(support::Random& random, Int a_lo, Int a_hi, Int& b_lo, Int& b_hi) {
  const Int size = random.between(0, 4096);
  const Int gap = random.between(-1, 1);
  switch (random.between(0, 2)) {
    case 0:  // b after a
      b_lo = clamped(a_hi + gap);
      b_hi = clamped(b_lo + size);
      break;
    case 1:  // b before a
      b_hi = clamped(a_lo - gap);
      b_lo = clamped(b_hi - size);
      break;
    default:  // b around a's low side
      b_lo = clamped(a_lo - size);
      b_hi = clamped(a_lo + size);
      break;
  }
}

// Built to touch or nearly touch: every axis is at most one sub-metre from a shared face.
Box near_box(support::Random& random, const Box& a) {
  Box b{};
  place_axis(random, a.lo.x, a.hi.x, b.lo.x, b.hi.x);
  place_axis(random, a.lo.y, a.hi.y, b.lo.y, b.hi.y);
  place_axis(random, a.lo.z, a.hi.z, b.lo.z, b.hi.z);
  return b;
}

std::string show_pair(const Box& a, const Box& b) {
  return hop::geom::vec_text(a.lo) + " " + hop::geom::vec_text(a.hi) + " against " +
         hop::geom::vec_text(b.lo) + " " + hop::geom::vec_text(b.hi);
}

}  // namespace

// The game's touch and raylib's are one test.
TEST(touch_matches_raylib) {
  support::Random random(2026);
  int touching = 0;
  for (int i = 0; i < pair_count; ++i) {
    Box a{};
    Box b{};
    if (i % 2 == 0) {
      a = ordered_box(random);
      b = near_box(random, a);
    } else if (i % 4 == 1) {
      a = any_box(random);
      b = any_box(random);
    } else {
      a = ordered_box(random);
      b = shifted(a, {random.between(-4096, 4096), random.between(-4096, 4096), random.between(-4096, 4096)});
      b = {{clamped(b.lo.x), clamped(b.lo.y), clamped(b.lo.z)},
           {clamped(b.hi.x), clamped(b.hi.y), clamped(b.hi.z)}};
    }
    const bool ours = hop::geom::touches(a, b);
    touching += ours;
    if (ours != hop::screen::check_collision_boxes(a, b)) {
      harness::fail("pair " + std::to_string(i) + ": touches is " + harness::show(ours) +
                    ", raylib disagrees: " + show_pair(a, b));
    }
    if (ours != hop::geom::touches(b, a)) {
      harness::fail("pair " + std::to_string(i) + ": touches is not symmetric: " + show_pair(a, b));
    }
  }
  std::printf("  %d pairs, %d touching\n", pair_count, touching);
  CHECK(pair_count >= 2000);
  // Both answers must be well represented, or agreement means little.
  CHECK(touching > pair_count / 5);
  CHECK(pair_count - touching > pair_count / 5);
}

// The @cases of touches, through the library.
TEST(touch_cases_through_raylib) {
  CHECK_EQ(hop::screen::check_collision_boxes({{0, 0, 0}, {2, 2, 2}}, {{2, 0, 0}, {4, 2, 2}}), true);
  CHECK_EQ(hop::screen::check_collision_boxes({{0, 0, 0}, {2, 2, 2}}, {{3, 0, 0}, {4, 2, 2}}), false);
}
