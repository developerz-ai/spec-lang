// Projected by hand from the app's rules list (src/apps/cpp.slg) over todo.board (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <optional>
#include <string>
#include <vector>

#include "todo/board/errors.hpp"
#include "todo/board/world.hpp"
#include "todo/cli/command.hpp"

namespace todo::cli {

// What one command did to a world, ready for the store and the terminal.
struct Applied {
  std::optional<board::World> written;  // set only for a writing rule that succeeded
  std::vector<std::string> lines;
  std::optional<board::RuleError> refused;
};

[[nodiscard]] Applied apply(const Command& command, const board::World& world);

[[nodiscard]] constexpr Access access_of(const Command& command) noexcept {
  return spec_of(id_of(command)).access;
}

}  // namespace todo::cli
