// Projected by hand from Hold, pad_of, meadow_win, ridge_win (src/hop/tests/runs.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <array>
#include <string_view>

#include "hop/model/pad.hpp"

namespace scripts {

struct Hold {
  int ticks;  // 1..=10000
  std::string_view keys;
};

// One letter per Pad field, as the game vocabulary reads a script: L R F B J, or - for none.
constexpr hop::model::Pad pad_of(std::string_view keys) {
  return {
      .left = keys.contains('L'),
      .right = keys.contains('R'),
      .forward = keys.contains('F'),
      .back = keys.contains('B'),
      .jump = keys.contains('J'),
  };
}

// Run right, with four held jumps.
inline constexpr std::array<Hold, 9> meadow_win{{
    {75, "R"},
    {20, "RJ"},
    {41, "R"},
    {20, "RJ"},
    {35, "R"},
    {20, "RJ"},
    {23, "R"},
    {20, "RJ"},
    {62, "R"},
}};

// Found by a search over pad scripts: wait for the mover, ride it, hop the gem, then the high ground.
inline constexpr std::array<Hold, 42> ridge_win{{
    {24, "R"},  {8, "-"},   {8, "R"},  {16, "RJ"}, {16, "R"},  {8, "-"}, {8, "L"},  {8, "R"},   {8, "L"},
    {8, "R"},   {8, "L"},   {8, "R"},  {8, "L"},   {8, "R"},   {8, "L"}, {8, "R"},  {16, "RJ"}, {24, "R"},
    {16, "RJ"}, {8, "-"},   {8, "R"},  {16, "-"},  {16, "J"},  {8, "-"}, {16, "R"}, {8, "J"},   {8, "L"},
    {24, "R"},  {16, "RJ"}, {32, "R"}, {8, "RJ"},  {8, "J"},   {8, "R"}, {8, "-"},  {16, "RJ"}, {32, "R"},
    {16, "RJ"}, {16, "R"},  {8, "-"},  {32, "R"},  {16, "RJ"}, {8, "R"},
}};

}  // namespace scripts
