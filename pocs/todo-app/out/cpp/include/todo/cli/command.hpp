// Projected by hand from the parameters of rule add, list, done, reopen, rename, remove, clear, stats (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <optional>
#include <set>
#include <variant>

#include "todo/cli/params.hpp"
#include "todo/model/types.hpp"

namespace todo::cli {

struct Add {
  model::Title title;
  model::Priority priority = model::Priority::Normal;
  std::set<model::Tag> tags;
};
struct List {
  bool all = false;
  std::optional<model::Tag> tag;
};
struct Done {
  model::TaskNo no;
};
struct Reopen {
  model::TaskNo no;
};
struct Rename {
  model::TaskNo no;
  model::Title title;
};
struct Remove {
  model::TaskNo no;
};
struct Clear {};
struct Stats {};

// Alternative order matches CommandId.
using Command = std::variant<Add, List, Done, Reopen, Rename, Remove, Clear, Stats>;

[[nodiscard]] constexpr CommandId id_of(const Command& c) noexcept {
  return static_cast<CommandId>(c.index());
}

}  // namespace todo::cli
