// Projected by hand from the parameters of every @on(cli(...)) rule in todo.board (src/todo/board.slg), derived per the cli vocabulary (docs/cli-vocabulary.md). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <array>
#include <cstdint>
#include <optional>
#include <span>
#include <string_view>
#include <utility>

#include "todo/model/types.hpp"

namespace todo::cli {

// The parameter types a command line can carry, each checked at the edge.
enum class ValueType : std::uint8_t { TaskNo, Title, Tag, Priority };

[[nodiscard]] constexpr std::string_view type_name(ValueType t) noexcept {
  switch (t) {
    case ValueType::TaskNo:
      return "TaskNo";
    case ValueType::Title:
      return "Title";
    case ValueType::Tag:
      return "Tag";
    case ValueType::Priority:
      return "Priority";
  }
  return "";
}

// type Priority = Low | Normal | High: a sum with no data is its variant names in lower case.
inline constexpr std::array<std::pair<std::string_view, model::Priority>, 3> priority_names{{
    {"low", model::Priority::Low},
    {"normal", model::Priority::Normal},
    {"high", model::Priority::High},
}};

// Bool = false -> Switch; Option[T] = none -> Option; Set[T] = ... -> Repeated;
// a sum with a default -> Single (one value, like Option: the vocabulary has no repeat for it).
enum class FlagKind : std::uint8_t { Switch, Option, Single, Repeated };

struct Positional {
  std::string_view name;
  ValueType type;
};

struct Flag {
  std::string_view name;
  FlagKind kind;
  ValueType type;  // unused for a Switch
};

// From @uses: tasks.read only, or tasks.write.
enum class Access : std::uint8_t { Read, Write };

// Same order as the app's rules list and as the Command variant.
enum class CommandId : std::uint8_t { Add, List, Done, Reopen, Rename, Remove, Clear, Stats };

struct CommandSpec {
  CommandId id;
  std::string_view name;
  std::span<const Positional> positionals;
  std::span<const Flag> flags;
  Access access;
};

namespace table {
inline constexpr std::array<Positional, 1> title{{{"title", ValueType::Title}}};
inline constexpr std::array<Positional, 1> no{{{"no", ValueType::TaskNo}}};
inline constexpr std::array<Positional, 2> no_title{{{"no", ValueType::TaskNo}, {"title", ValueType::Title}}};
inline constexpr std::array<Flag, 2> add_flags{{
    {"priority", FlagKind::Single, ValueType::Priority},
    {"tags", FlagKind::Repeated, ValueType::Tag},
}};
inline constexpr std::array<Flag, 2> list_flags{{
    {"all", FlagKind::Switch, ValueType::Tag},
    {"tag", FlagKind::Option, ValueType::Tag},
}};
}  // namespace table

inline constexpr std::array<CommandSpec, 8> commands{{
    {CommandId::Add, "add", table::title, table::add_flags, Access::Write},
    {CommandId::List, "list", {}, table::list_flags, Access::Read},
    {CommandId::Done, "done", table::no, {}, Access::Write},
    {CommandId::Reopen, "reopen", table::no, {}, Access::Write},
    {CommandId::Rename, "rename", table::no_title, {}, Access::Write},
    {CommandId::Remove, "remove", table::no, {}, Access::Write},
    {CommandId::Clear, "clear", {}, {}, Access::Write},
    {CommandId::Stats, "stats", {}, {}, Access::Read},
}};

[[nodiscard]] constexpr const CommandSpec* find_command(std::string_view name) noexcept {
  for (const CommandSpec& c : commands) {
    if (c.name == name) {
      return &c;
    }
  }
  return nullptr;
}

[[nodiscard]] constexpr const CommandSpec& spec_of(CommandId id) noexcept {
  return commands[static_cast<std::size_t>(id)];
}

}  // namespace todo::cli
