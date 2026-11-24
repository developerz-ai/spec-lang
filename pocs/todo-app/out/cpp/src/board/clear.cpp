// Projected by hand from rule clear (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include <string>
#include <utility>

#include "todo/board/rules.hpp"

namespace todo::board {

Outcome<std::size_t> clear(const World& w) {
  World next = w;
  const std::size_t gone =
      std::erase_if(next.tasks, [](const auto& entry) { return entry.second.status == model::Status::Done; });
  return {std::move(next), {"cleared " + std::to_string(gone)}, gone};
}

}  // namespace todo::board
