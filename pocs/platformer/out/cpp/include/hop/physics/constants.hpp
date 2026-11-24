// Projected by hand from body_half, crawler_half, gem_half, goal_half and the speed constants (src/hop/physics.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include "hop/geom/vec3.hpp"

namespace hop::physics {

using geom::Int;

// Half the width, the full height and half the depth of the hero: 0.5 m by 0.75 m by 0.5 m.
inline constexpr geom::Vec3 body_half{64, 192, 64};
inline constexpr geom::Vec3 crawler_half{96, 128, 96};
inline constexpr Int gem_half = 64;
inline constexpr geom::Vec3 goal_half{64, 768, 64};

// 3.75 m/s flat out; about the same speed on a diagonal.
inline constexpr Int run_speed = 16;
inline constexpr Int diagonal_speed = 11;
inline constexpr Int ground_accel = 2;
inline constexpr Int air_accel = 1;
inline constexpr Int gravity = 2;
// Never more than 48 sub-metres in a tick, less than the thinnest solid.
inline constexpr Int max_fall = 48;
inline constexpr Int jump_speed = 40;
// Letting go of jump early cuts the rise: a short hop.
inline constexpr Int hop_cut = 16;
// A jump still counts for a few ticks after running off an edge.
inline constexpr Int coyote_ticks = 6;
inline constexpr Int stomp_bounce = 24;
// A stomp lands on the upper half of a crawler.
inline constexpr Int stomp_line = 64;
inline constexpr Int kill_y = -2048;

}  // namespace hop::physics
