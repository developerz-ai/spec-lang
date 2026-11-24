// Projected by hand from LevelNo, level (src/hop/levels.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <optional>

#include "hop/levels/meadow.hpp"
#include "hop/levels/ridge.hpp"

namespace hop::levels {

// LevelNo = 1..=2. Only make() builds one, so a LevelNo in hand is always in range.
class LevelNo {
 public:
  static constexpr std::optional<LevelNo> make(geom::Int n) {
    return n >= 1 && n <= 2 ? std::optional<LevelNo>(LevelNo(n)) : std::nullopt;
  }
  constexpr geom::Int value() const { return value_; }
  friend constexpr bool operator==(const LevelNo&, const LevelNo&) = default;

 private:
  constexpr explicit LevelNo(geom::Int n) : value_(n) {}
  geom::Int value_;
};

inline constexpr LevelNo first_level = *LevelNo::make(1);
inline constexpr LevelNo second_level = *LevelNo::make(2);

constexpr const model::Level& level(LevelNo n) { return n.value() == 1 ? meadow : ridge; }

}  // namespace hop::levels
