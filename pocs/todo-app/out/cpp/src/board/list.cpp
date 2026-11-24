// Projected by hand from rule list, fn has_tag (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include <algorithm>
#include <vector>

#include "todo/board/rules.hpp"
#include "todo/render/render.hpp"

namespace todo::board {

bool has_tag(const model::Task& t, const std::optional<model::Tag>& tag) {
  return !tag.has_value() || t.tags.contains(*tag);
}

Lines list(const World& w, bool all, const std::optional<model::Tag>& tag) {
  std::vector<const model::Task*> shown;
  shown.reserve(w.tasks.size());
  for (const auto& [no, t] : w.tasks) {
    if ((all || t.status == model::Status::Open) && has_tag(t, tag)) {
      shown.push_back(&t);
    }
  }
  if (shown.empty()) {
    return {"nothing to do"};
  }
  // rank ends in the task number, which is unique, so the order is total and stability moot.
  std::ranges::sort(shown, {}, [](const model::Task* t) { return render::rank(*t); });
  Lines lines;
  lines.reserve(shown.size());
  for (const model::Task* t : shown) {
    lines.push_back(render::line(*t));
  }
  return lines;
}

}  // namespace todo::board
