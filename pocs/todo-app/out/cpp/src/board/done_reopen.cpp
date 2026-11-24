// Projected by hand from rule done, rule reopen (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include <utility>

#include "todo/board/rules.hpp"
#include "todo/render/render.hpp"

namespace todo::board {
namespace {

// @guard(no in tasks else NoTask, tasks[no].status == from else wrong), then status = to.
Result<Nothing> move(const World& w, model::TaskNo no, model::Status from, model::Status to,
                     RuleError wrong) {
  const auto found = w.tasks.find(no);
  if (found == w.tasks.end()) {
    return std::unexpected(RuleError::NoTask);
  }
  if (found->second.status != from) {
    return std::unexpected(wrong);
  }
  World next = w;
  model::Task& t = next.tasks.at(no);
  t.status = to;
  std::string shown = render::line(t);
  return Outcome<Nothing>{std::move(next), {std::move(shown)}, {}};
}

}  // namespace

Result<Nothing> done(const World& w, model::TaskNo no) {
  return move(w, no, model::Status::Open, model::Status::Done, RuleError::AlreadyDone);
}

Result<Nothing> reopen(const World& w, model::TaskNo no) {
  return move(w, no, model::Status::Done, model::Status::Open, RuleError::NotDone);
}

}  // namespace todo::board
