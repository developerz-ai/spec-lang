// Projected by hand from state tasks, state next_no (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <map>

#include "todo/model/types.hpp"

namespace todo::board {

// Every state of todo.board, as one value. A rule takes one and returns the next.
struct World {
  std::map<model::TaskNo, model::Task> tasks;
  model::TaskNo next_no = model::TaskNo::first();

  bool operator==(const World&) const = default;
};

}  // namespace todo::board
