// Seeded generation for property tests and law traces: splitmix64, the same sequence on every machine.
#pragma once

#include <array>
#include <cstdint>
#include <string_view>

namespace rt::random {

class Prng {
 public:
  explicit Prng(std::uint64_t seed) : state_(seed) {}
  std::uint64_t next() {
    std::uint64_t z = (state_ += 0x9E3779B97F4A7C15ULL);
    z = (z ^ (z >> 30U)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27U)) * 0x94D049BB133111EBULL;
    return z ^ (z >> 31U);
  }
  // A value in [0, n).
  std::uint64_t below(std::uint64_t n) { return next() % n; }
  bool coin() { return below(2) == 0; }

 private:
  std::uint64_t state_;
};

// Candidate texts for a refined Text type; a generator keeps those its claim admits.
inline constexpr std::array<std::string_view, 16> texts{
    "milk", "  spaced  ", "café ☕",   "say \"hi\"", "back\\slash", "\U0001F600", "a b\u0085c", "--all",
    "home", "work",       "deep-work", "a",          "42",          "x",          "ship it",      ""};

}  // namespace rt::random
