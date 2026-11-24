// Projected by hand from axis, walk_target, facing (src/hop/physics.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include "hop/model/pad.hpp"
#include "hop/physics/constants.hpp"

namespace hop::physics {

constexpr Int axis(bool neg, bool pos) { return (pos ? 1 : 0) - (neg ? 1 : 0); }

// x runs right, z runs toward the camera, so forward is minus z.
constexpr geom::Vec3 walk_target(const model::Pad& pad) {
  const Int dx = axis(pad.left, pad.right);
  const Int dz = axis(pad.forward, pad.back);
  const Int speed = dx != 0 && dz != 0 ? diagonal_speed : run_speed;
  return {dx * speed, 0, dz * speed};
}

// The hero faces where the pad points, and keeps facing there when it lets go.
constexpr Int facing(const model::Pad& pad, Int was) {
  const Int dx = axis(pad.left, pad.right);
  const Int dz = axis(pad.forward, pad.back);
  if (dx == 1) {
    return dz == -1 ? 45 : dz == 1 ? 315 : 0;
  }
  if (dx == -1) {
    return dz == -1 ? 135 : dz == 1 ? 225 : 180;
  }
  return dz == -1 ? 90 : dz == 1 ? 270 : was;
}

}  // namespace hop::physics
