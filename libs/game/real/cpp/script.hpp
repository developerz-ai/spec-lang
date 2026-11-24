// The `game` library's runtime for C++ (libs/game/real/cpp): the input script of `run`, `draw` and `shot`.
// The tick rule's one parameter is a record of Bool fields; each field is a key, written as its first letter
// in upper case. A line is `<ticks> <keys>`: hold these keys for that many ticks; `-` holds none; `#` starts
// a comment.
#pragma once

#include <cstdint>
#include <expected>
#include <string>
#include <string_view>
#include <vector>

#include "rt/meta.hpp"

namespace lib::game {

struct Hold {
  std::uint32_t ticks = 0;
  // The tick rule's argument: the record, each field true when its key is held.
  rt::Data controls;
};

struct Script {
  std::vector<Hold> holds;
};

inline constexpr std::uint32_t kMaxLineTicks = 100'000;
inline constexpr std::uint64_t kMaxTotalTicks = 1'000'000;

// The whole input, checked before the first tick. The error is the message without "error: ".
std::expected<Script, std::string> read_script(const rt::meta::Type& controls, std::string_view input);

}  // namespace lib::game
