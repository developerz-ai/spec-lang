#include "rt/random.hpp"

#include <array>
#include <bit>

#include "rt/text.hpp"

namespace rt::test {

std::uint64_t Rng::next() {
  state_ += 0x9E3779B97F4A7C15ULL;
  std::uint64_t z = state_;
  z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
  z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
  return z ^ (z >> 31);
}

Int Rng::between(Int lo, Int hi) {
  if (hi <= lo) return lo;
  const uint128 span = static_cast<uint128>(hi.value() - lo.value()) + 1;
  const uint128 high = next();
  const uint128 r = ((high << 64) | next()) % span;
  return Int::wide(lo.value() + static_cast<int128>(r));
}

std::uint64_t seed_for(std::uint64_t base, std::string_view name, std::uint64_t index) {
  std::uint64_t h = 0xcbf29ce484222325ULL;
  for (const char c : name) {
    h ^= static_cast<unsigned char>(c);
    h *= 0x00000100000001b3ULL;
  }
  Rng rng{h ^ std::rotl(base, 17) ^ (index * 0x9E3779B97F4A7C15ULL)};
  return rng.next();
}

// Small numbers most of the time, then boundaries a machine word breaks at.
Int draw_int(Rng& rng) {
  static const std::array<Int, 11> edges = {
      0_i,
      1_i,
      -1_i,
      2147483647_i,
      2147483648_i,
      9007199254740992_i,
      9007199254740993_i,
      9223372036854775807_i,
      9223372036854775808_i,
      -9223372036854775808_i,
      18446744073709551616_i,
  };
  const Int big = 4611686018427387904_i;  // 2^62
  const auto band = rng.below(100);
  if (band < 35) return rng.between(0_i, 10_i);
  if (band < 55) return rng.between(11_i, 100_i);
  if (band < 70) return rng.between(-10_i, -1_i);
  if (band < 80) return edges[rng.below(edges.size())];
  if (band < 90) return rng.between(101_i, big);
  return -rng.between(11_i, big);
}

// Text from one of four alphabets: shell-safe words, printable ASCII, a mix with Unicode space, controls
// and astral characters, or empty.
Text draw_text(Rng& rng) {
  static const std::string_view word = "abcdefghijklmnopqrstuvwxyz0123456789-";
  static const std::array<char32_t, 22> mix = {U'a',    U'Z',   U' ', U'\t', U'\n',  0xE9,   0xDF, 0x4E2D,
                                               0x1F600, 0x3000, 0x85, 0xA0,  0x200B, 0xFEFF, 0x7F, 0x1,
                                               U'"',    U'\\',  U'{', U'}',  U'#',   U'-'};
  Text out;
  switch (rng.below(4)) {
    case 0:
      for (auto n = 1 + rng.below(8); n > 0; --n) out.push_back(word[rng.below(word.size())]);
      break;
    case 1:
      for (auto n = rng.below(13); n > 0; --n) out.push_back(static_cast<char>(32 + rng.below(95)));
      break;
    case 2:
      for (auto n = rng.below(9); n > 0; --n) unicode::append(out, mix[rng.below(mix.size())]);
      break;
    default:
      break;
  }
  return out;
}

Int draw_range(Rng& rng, Int lo, Int hi) {
  if (rng.chance(1, 5)) return rng.chance(1, 2) ? lo : hi;
  return rng.between(lo, hi);
}

}  // namespace rt::test
