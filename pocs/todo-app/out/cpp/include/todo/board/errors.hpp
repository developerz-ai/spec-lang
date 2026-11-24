// Projected by hand from the guard errors NoTask, AlreadyDone, NotDone (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <cstdint>
#include <string_view>

namespace todo::board {

enum class RuleError : std::uint8_t { NoTask, AlreadyDone, NotDone };

[[nodiscard]] constexpr std::string_view name(RuleError e) noexcept {
  switch (e) {
    case RuleError::NoTask:
      return "NoTask";
    case RuleError::AlreadyDone:
      return "AlreadyDone";
    case RuleError::NotDone:
      return "NotDone";
  }
  return "NoTask";
}

}  // namespace todo::board
