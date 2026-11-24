// Projected by hand from "ASCII digits, leading zeros allowed" for --level, --at and a script's ticks (docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <cstdint>
#include <optional>
#include <string_view>

namespace game::cli {

// The value of a non-empty run of ASCII digits, exact up to `ceiling` and `ceiling` for anything
// larger, of any length. Empty text or any other character: nothing.
constexpr std::optional<std::uint32_t> digits_up_to(std::string_view text, std::uint32_t ceiling) {
  if (text.empty()) {
    return std::nullopt;
  }
  std::uint64_t value = 0;
  for (const char c : text) {
    if (c < '0' || c > '9') {
      return std::nullopt;
    }
    if (value < ceiling) {
      value = value * 10 + static_cast<std::uint64_t>(c - '0');
    }
  }
  return value < ceiling ? static_cast<std::uint32_t>(value) : ceiling;
}

}  // namespace game::cli
