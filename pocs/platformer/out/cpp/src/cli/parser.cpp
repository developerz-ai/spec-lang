// Projected by hand from the argument rules of the game vocabulary (docs/contract.md section 2, "Parsing, in this order") for app SkyhopCpp (src/apps/cpp.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "game/cli/parser.hpp"

#include <algorithm>
#include <array>
#include <cstddef>

#include "game/cli/digits.hpp"

namespace game::cli {
namespace {

enum class Flag : std::uint8_t { Level, At, Out };

struct FlagSpec {
  std::string_view name;
  Flag flag;
};
struct CommandSpec {
  std::string_view name;
  Command command;
  std::span<const FlagSpec> flags;
};

// The table the vocabulary derives: every command starts a level, draw and shot stop at a tick,
// shot can write its picture.
constexpr std::array<FlagSpec, 3> all_flags{
    {{"--level", Flag::Level}, {"--at", Flag::At}, {"--out", Flag::Out}}};
constexpr std::span<const FlagSpec> flags(all_flags);
constexpr std::array<CommandSpec, 4> commands{{
    {"play", Command::Play, flags.first(1)},
    {"run", Command::Run, flags.first(1)},
    {"draw", Command::Draw, flags.first(2)},
    {"shot", Command::Shot, flags.first(3)},
}};

std::string quoted(std::string_view text) { return std::string("\"").append(text).append("\""); }

UsageError flag_error(std::string_view name, std::string_view why) {
  return {std::string("error: ").append(name).append(why)};
}

}  // namespace

Parsed parse(std::span<const std::string_view> args) {
  if (args.empty()) {
    return NoArguments{};
  }
  if (std::ranges::any_of(args, [](std::string_view a) { return a == "--help" || a == "-h"; })) {
    return HelpAsked{};
  }
  const auto spec = std::ranges::find(commands, args[0], &CommandSpec::name);
  if (spec == commands.end()) {
    return UsageError{"error: unknown command " + quoted(args[0])};
  }

  Invocation invocation{spec->command, hop::levels::first_level, std::nullopt, std::nullopt};
  std::array<bool, all_flags.size()> seen{};
  for (std::size_t i = 1; i < args.size(); ++i) {
    const std::string_view arg = args[i];
    if (!arg.starts_with("--")) {
      return UsageError{"error: unexpected argument " + quoted(arg)};
    }
    const auto flag = std::ranges::find(spec->flags, arg, &FlagSpec::name);
    if (flag == spec->flags.end()) {
      return UsageError{std::string("error: unknown flag ").append(arg)};
    }
    if (i + 1 == args.size()) {
      return flag_error(arg, " needs a value");
    }
    bool& given = seen[static_cast<std::size_t>(flag->flag)];
    if (given) {
      return flag_error(arg, " given twice");
    }
    given = true;
    const std::string_view value = args[++i];
    switch (flag->flag) {
      case Flag::Level: {
        // 1 or 2; a ceiling of 3 keeps any longer number out of range without overflow.
        const auto n = digits_up_to(value, 3);
        const auto level = n ? hop::levels::LevelNo::make(static_cast<hop::geom::Int>(*n)) : std::nullopt;
        if (!level) {
          return flag_error(arg, ": not a LevelNo");
        }
        invocation.level = *level;
        break;
      }
      case Flag::At: {
        const auto tick = digits_up_to(value, tick_ceiling);
        if (!tick) {
          return flag_error(arg, ": not a tick");
        }
        invocation.at = *tick;
        break;
      }
      case Flag::Out:
        if (value.empty()) {
          return flag_error(arg, ": not a path");
        }
        invocation.out = std::string(value);
        break;
    }
  }
  return invocation;
}

}  // namespace game::cli
