// Projected by hand from rule add (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include <string>
#include <utility>

#include "todo/board/rules.hpp"

namespace todo::board {

Outcome<model::TaskNo> add(const World& w, model::Title title, model::Priority priority,
                           std::set<model::Tag> tags) {
  const model::TaskNo no = w.next_no;
  World next = w;
  next.tasks.insert_or_assign(no, model::Task{.no = no,
                                              .title = std::move(title),
                                              .status = model::Status::Open,
                                              .priority = priority,
                                              .tags = std::move(tags)});
  next.next_no = no.next();
  return {std::move(next), {"added " + std::to_string(no.value())}, no};
}

}  // namespace todo::board
