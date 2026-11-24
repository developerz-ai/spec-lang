// Projected by hand from "values are generated from their types", seeded and replayable (docs/idea/01-language-card.md, Tests) for the laws and property tests of hop (src/hop/laws.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <cstdint>

namespace support {

// splitmix64: a fixed, seeded stream, the same on every machine.
class Random {
 public:
  explicit Random(std::uint64_t seed) : state_(seed) {}

  std::uint64_t next() {
    state_ += 0x9E3779B97F4A7C15ULL;
    std::uint64_t z = state_;
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
    return z ^ (z >> 31);
  }
  // A value in lo..=hi.
  std::int32_t between(std::int32_t lo, std::int32_t hi) {
    const auto width = static_cast<std::uint64_t>(static_cast<std::int64_t>(hi) - lo + 1);
    return static_cast<std::int32_t>(lo + static_cast<std::int64_t>(next() % width));
  }
  // True once in n.
  bool one_in(std::uint32_t n) { return next() % n == 0; }

 private:
  std::uint64_t state_;
};

}  // namespace support
