// Projected by hand from rule rename (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include <utility>

#include "todo/board/rules.hpp"
#include "todo/render/render.hpp"

namespace todo::board {

Result<Nothing> rename(const World& w, model::TaskNo no, model::Title title) {
  if (!w.tasks.contains(no)) {
    return std::unexpected(RuleError::NoTask);
  }
  World next = w;
  model::Task& t = next.tasks.at(no);
  t.title = std::move(title);
  std::string shown = render::line(t);
  return Outcome<Nothing>{std::move(next), {std::move(shown)}, {}};
}

}  // namespace todo::board
