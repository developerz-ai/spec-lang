// Projected by hand from rule remove (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include <string>
#include <utility>

#include "todo/board/rules.hpp"

namespace todo::board {

Result<Nothing> remove(const World& w, model::TaskNo no) {
  if (!w.tasks.contains(no)) {
    return std::unexpected(RuleError::NoTask);
  }
  World next = w;
  next.tasks.erase(no);
  return Outcome<Nothing>{std::move(next), {"removed " + std::to_string(no.value())}, {}};
}

}  // namespace todo::board
