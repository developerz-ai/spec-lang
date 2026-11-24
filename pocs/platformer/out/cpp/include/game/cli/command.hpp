// Projected by hand from the commands play, run, draw, shot the game vocabulary derives (docs/game-vocabulary.md) for app SkyhopCpp (src/apps/cpp.slg); --level from rule start(level: LevelNo) (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <cstdint>
#include <optional>
#include <string>
#include <variant>

#include "hop/levels/levels.hpp"

namespace game::cli {

enum class Command : std::uint8_t { Play, Run, Draw, Shot };

// A script holds at most 1,000,000 ticks, so every --at at or above that means "the end of the
// script" and the exact value of a longer number never has to be kept.
inline constexpr std::uint32_t tick_ceiling = 1'000'000;

struct Invocation {
  Command command;
  hop::levels::LevelNo level = hop::levels::first_level;
  std::optional<std::uint32_t> at;  // absent: the end of the script; saturates at tick_ceiling
  std::optional<std::string> out;
  friend bool operator==(const Invocation&, const Invocation&) = default;
};

// Help was asked for: to standard output, exit 0.
struct HelpAsked {
  friend bool operator==(const HelpAsked&, const HelpAsked&) = default;
};
// No arguments: help to standard error, exit 2.
struct NoArguments {
  friend bool operator==(const NoArguments&, const NoArguments&) = default;
};
// One line for standard error, without the newline; exit 2.
struct UsageError {
  std::string message;
  friend bool operator==(const UsageError&, const UsageError&) = default;
};

using Parsed = std::variant<Invocation, HelpAsked, NoArguments, UsageError>;

}  // namespace game::cli
