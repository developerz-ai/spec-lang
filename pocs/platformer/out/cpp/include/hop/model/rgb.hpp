// Projected by hand from Rgb (src/hop/model.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <cstdint>

namespace hop::model {

// Each channel is 0..=255, so one byte holds it: the layout of raylib's Color without the alpha.
struct Rgb {
  std::uint8_t r;
  std::uint8_t g;
  std::uint8_t b;
  friend constexpr bool operator==(const Rgb&, const Rgb&) = default;
};

}  // namespace hop::model
