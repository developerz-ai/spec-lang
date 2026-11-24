// Projected by hand from the frame hash of shot (docs/contract.md section 2): FNV-1a 64. Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <cstdint>
#include <span>
#include <string>

namespace game::app {

constexpr std::uint64_t fnv1a64(std::span<const std::uint8_t> bytes) {
  std::uint64_t hash = 0xcbf29ce484222325ULL;
  for (const std::uint8_t b : bytes) {
    hash ^= b;
    hash *= 0x100000001b3ULL;
  }
  return hash;
}

// 16 lower-case hex digits.
std::string hex16(std::uint64_t value);

}  // namespace game::app
