// Projected by hand from rule stats (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include <algorithm>
#include <string>

#include "todo/board/rules.hpp"

namespace todo::board {

std::string stats_line(std::size_t open, std::size_t done) {
  return std::to_string(open) + " open, " + std::to_string(done) + " done";
}

Lines stats(const World& w) {
  const auto open = static_cast<std::size_t>(std::ranges::count_if(
      w.tasks, [](const auto& entry) { return entry.second.status == model::Status::Open; }));
  return {stats_line(open, w.tasks.size() - open)};
}

}  // namespace todo::board
