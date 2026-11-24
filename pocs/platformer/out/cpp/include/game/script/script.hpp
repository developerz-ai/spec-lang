// Projected by hand from the input script the game vocabulary derives from rule step(pad: Pad) (docs/game-vocabulary.md; src/hop/world.slg, src/hop/model.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <array>
#include <cstdint>
#include <vector>

#include "hop/model/pad.hpp"

namespace game::script {

inline constexpr std::uint32_t max_line_ticks = 100'000;
inline constexpr std::uint32_t max_total_ticks = 1'000'000;

// One letter per Pad field: the field's first letter in upper case, in declared order.
struct Key {
  char letter;
  bool hop::model::Pad::* field;
};
inline constexpr std::array<Key, 5> keys{{
    {'L', &hop::model::Pad::left},
    {'R', &hop::model::Pad::right},
    {'F', &hop::model::Pad::forward},
    {'B', &hop::model::Pad::back},
    {'J', &hop::model::Pad::jump},
}};

// One line of the script: the same controls for a count of ticks.
struct Hold {
  std::uint32_t ticks;  // 1..=100000
  hop::model::Pad pad;
  friend constexpr bool operator==(const Hold&, const Hold&) = default;
};

// The script stays run-length encoded as written: a million ticks are at most a few lines, never
// a million pads.
struct Script {
  std::vector<Hold> holds;
  std::uint32_t total_ticks = 0;  // at most max_total_ticks
  friend bool operator==(const Script&, const Script&) = default;
};

}  // namespace game::script
