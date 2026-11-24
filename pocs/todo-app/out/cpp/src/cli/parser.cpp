// Projected by hand from the cli vocabulary's derivations (docs/cli-vocabulary.md) and docs/contract.md section 2. Stand-in for spec-lang build output; change the spec, not this file.
#include "todo/cli/parser.hpp"

#include <cstdint>
#include <limits>
#include <utility>
#include <vector>

namespace todo::cli {
namespace {

using Value = std::variant<model::TaskNo, model::Title, model::Tag, model::Priority>;

std::optional<Value> parse_value(ValueType type, std::string_view text) {
  switch (type) {
    case ValueType::TaskNo:
      return parse_task_no(text);
    case ValueType::Title:
      return model::Title::make(std::string(text));
    case ValueType::Tag:
      return model::Tag::make(text);
    case ValueType::Priority:
      return parse_priority(text);
  }
  return std::nullopt;
}

// Arguments bound to one command's table, every value already of its type.
struct Bound {
  std::vector<Value> positionals;
  std::vector<std::vector<Value>> flag_values;  // one entry per table flag
  std::vector<bool> flag_seen;
};

UsageError usage(std::string message) { return UsageError{std::move(message)}; }

std::string quoted(std::string_view s) {
  std::string out = "\"";
  out += s;
  out += '"';
  return out;
}

template <class T>
T take(const Value& v) {
  return std::get<T>(v);
}

std::optional<model::Tag> option_tag(const std::vector<Value>& values) {
  if (values.empty()) {
    return std::nullopt;
  }
  return take<model::Tag>(values.front());
}

Command build(CommandId id, const Bound& b) {
  switch (id) {
    case CommandId::Add: {
      Add add{.title = take<model::Title>(b.positionals[0]), .priority = model::Priority::Normal, .tags = {}};
      if (!b.flag_values[0].empty()) {
        add.priority = take<model::Priority>(b.flag_values[0].front());
      }
      for (const Value& v : b.flag_values[1]) {
        add.tags.insert(take<model::Tag>(v));  // repeats collapse
      }
      return add;
    }
    case CommandId::List:
      return List{.all = b.flag_seen[0], .tag = option_tag(b.flag_values[1])};
    case CommandId::Done:
      return Done{take<model::TaskNo>(b.positionals[0])};
    case CommandId::Reopen:
      return Reopen{take<model::TaskNo>(b.positionals[0])};
    case CommandId::Rename:
      return Rename{take<model::TaskNo>(b.positionals[0]), take<model::Title>(b.positionals[1])};
    case CommandId::Remove:
      return Remove{take<model::TaskNo>(b.positionals[0])};
    case CommandId::Clear:
      return Clear{};
    case CommandId::Stats:
      return Stats{};
  }
  return Stats{};
}

const Flag* find_flag(const CommandSpec& spec, std::string_view name, std::size_t& index) {
  for (std::size_t i = 0; i < spec.flags.size(); ++i) {
    if (spec.flags[i].name == name) {
      index = i;
      return &spec.flags[i];
    }
  }
  return nullptr;
}

bool wants_help(std::span<const std::string_view> args) {
  for (const std::string_view a : args) {
    if (a == "--") {
      return false;
    }
    if (a == "--help" || a == "-h") {
      return true;
    }
  }
  return false;
}

}  // namespace

std::optional<model::TaskNo> parse_task_no(std::string_view text) noexcept {
  if (text.empty()) {
    return std::nullopt;
  }
  constexpr std::uint64_t max = std::numeric_limits<std::uint64_t>::max();
  std::uint64_t value = 0;
  for (const char c : text) {
    if (c < '0' || c > '9') {
      return std::nullopt;
    }
    const auto digit = static_cast<std::uint64_t>(c - '0');
    value = (value > (max - digit) / 10) ? max : value * 10 + digit;
  }
  return model::TaskNo::make(value);
}

std::optional<model::Priority> parse_priority(std::string_view text) noexcept {
  for (const auto& [name, priority] : priority_names) {
    if (name == text) {
      return priority;
    }
  }
  return std::nullopt;
}

Parsed parse(std::span<const std::string_view> args) {
  if (args.empty()) {
    return HelpRequest{.to_stdout = false};
  }
  if (wants_help(args)) {
    return HelpRequest{.to_stdout = true};
  }
  const CommandSpec* spec = find_command(args[0]);
  if (spec == nullptr) {
    return usage("unknown command " + quoted(args[0]));
  }
  Bound bound{.positionals = {},
              .flag_values = std::vector<std::vector<Value>>(spec->flags.size()),
              .flag_seen = std::vector<bool>(spec->flags.size(), false)};
  bool flags_ended = false;
  for (std::size_t i = 1; i < args.size(); ++i) {
    const std::string_view arg = args[i];
    if (!flags_ended && arg == "--") {
      flags_ended = true;
      continue;
    }
    if (!flags_ended && arg.starts_with("--")) {
      std::size_t index = 0;
      const Flag* flag = find_flag(*spec, arg.substr(2), index);
      if (flag == nullptr) {
        return usage("unknown flag " + std::string(arg));
      }
      if (flag->kind == FlagKind::Switch) {
        if (bound.flag_seen[index]) {
          return usage(std::string(arg) + " given twice");
        }
        bound.flag_seen[index] = true;
        continue;
      }
      if (i + 1 == args.size()) {
        return usage(std::string(arg) + " needs a value");
      }
      if (bound.flag_seen[index] && flag->kind != FlagKind::Repeated) {
        return usage(std::string(arg) + " given twice");
      }
      const std::string_view text = args[++i];
      std::optional<Value> value = parse_value(flag->type, text);
      if (!value) {
        return usage(std::string(arg) + ": not a " + std::string(type_name(flag->type)));
      }
      bound.flag_seen[index] = true;
      bound.flag_values[index].push_back(std::move(*value));
      continue;
    }
    const std::size_t at = bound.positionals.size();
    if (at == spec->positionals.size()) {
      return usage("unexpected argument " + quoted(arg));
    }
    const Positional& p = spec->positionals[at];
    std::optional<Value> value = parse_value(p.type, arg);
    if (!value) {
      return usage("<" + std::string(p.name) + ">: not a " + std::string(type_name(p.type)));
    }
    bound.positionals.push_back(std::move(*value));
  }
  if (bound.positionals.size() < spec->positionals.size()) {
    return usage("missing <" + std::string(spec->positionals[bound.positionals.size()].name) + ">");
  }
  return build(spec->id, bound);
}

}  // namespace todo::cli
