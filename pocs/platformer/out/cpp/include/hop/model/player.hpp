// Projected by hand from Mode, Player (src/hop/model.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <cstdint>
#include <optional>

#include "hop/geom/vec3.hpp"

namespace hop::model {

enum class Mode : std::uint8_t { Playing, Won, Lost };

// riding names the mover the feet are on, so the body moves with it.
struct Player {
  geom::Vec3 at;
  geom::Vec3 vel = {0, 0, 0};
  bool grounded = false;
  geom::Int coyote = 0;  // 0..=6
  bool jump_held = false;
  std::optional<geom::Int> riding = std::nullopt;
  geom::Int yaw = 0;  // 0..=359
  friend constexpr bool operator==(const Player&, const Player&) = default;
};

}  // namespace hop::model
