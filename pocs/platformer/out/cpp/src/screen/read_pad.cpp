// Projected by hand from key_right, key_left, key_down_arrow, key_up, key_space, key_a, key_d, key_s, key_w, read_pad (src/hop/screen.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "edge.hpp"
#include "hop/screen/screen.hpp"

namespace hop::screen {
namespace {

constexpr geom::Int key_right = 262;
constexpr geom::Int key_left = 263;
constexpr geom::Int key_down_arrow = 264;
constexpr geom::Int key_up = 265;
constexpr geom::Int key_space = 32;
constexpr geom::Int key_a = 65;
constexpr geom::Int key_d = 68;
constexpr geom::Int key_s = 83;
constexpr geom::Int key_w = 87;

}  // namespace

model::Pad read_pad() {
  using edge::key_down;
  return {
      .left = key_down(key_left) || key_down(key_a),
      .right = key_down(key_right) || key_down(key_d),
      .forward = key_down(key_up) || key_down(key_w),
      .back = key_down(key_down_arrow) || key_down(key_s),
      .jump = key_down(key_space),
  };
}

}  // namespace hop::screen
