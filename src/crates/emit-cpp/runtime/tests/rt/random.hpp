// The generator of `spec-lang test`, step for step, so a property or a law here draws the engine's inputs:
// SplitMix64, seeds from FNV-1a of the check's name, and base values per type.
#pragma once

#include <cstdint>
#include <string_view>

#include "rt/value.hpp"

namespace rt::test {

class Rng {
 public:
  explicit Rng(std::uint64_t seed) : state_(seed) {}
  std::uint64_t next();
  // Uniform in 0..n (0 when n is 0).
  std::uint64_t below(std::uint64_t n) {
    return n == 0 ? 0 : next() % n;
  }
  // In lo..=hi: two draws make a 128-bit number, high first, modulo the span.
  Int between(Int lo, Int hi);
  bool chance(std::uint64_t num, std::uint64_t den) {
    return below(den) < num;
  }

 private:
  std::uint64_t state_;
};

// The seed of run `index` of the check `name`.
std::uint64_t seed_for(std::uint64_t base, std::string_view name, std::uint64_t index);

Int draw_int(Rng& rng);
Text draw_text(Rng& rng);
// An Int of lo..=hi: one of its ends one time in five.
Int draw_range(Rng& rng, Int lo, Int hi);

// How many values of one type may be tried before its claim counts as unmet.
inline constexpr int kTries = 400;

// A value kept when it conforms: half the time one from the pool when it has any, else a fresh one.
template <class T, class Base, class Ok>
T keep(Rng& rng, const Set<T>* pool, Base&& base, Ok&& conforms, const char* type) {
  for (int i = 0; i < kTries; ++i) {
    T value = [&]() -> T {
      if (pool != nullptr && !pool->empty() && rng.chance(1, 2)) {
        auto it = pool->begin();
        std::advance(it, static_cast<std::ptrdiff_t>(rng.below(pool->size())));
        return *it;
      }
      return base();
    }();
    if (conforms(value)) return value;
  }
  stop("E-GEN", Text{"no value of `"} + type + "` found in 400 tries");
}

template <class T, class F>
List<T> draw_list(Rng& rng, F&& item) {
  List<T> out;
  for (auto n = rng.below(4); n > 0; --n) out.push_back(item());
  return out;
}
template <class T, class F>
Set<T> draw_set(Rng& rng, F&& item) {
  Set<T> out;
  for (auto n = rng.below(4); n > 0; --n) out.insert(item());
  return out;
}
template <class K, class V, class F, class G>
Map<K, V> draw_map(Rng& rng, F&& key, G&& value) {
  Map<K, V> out;
  for (auto n = rng.below(4); n > 0; --n) {
    K k = key();
    out.insert_or_assign(std::move(k), value());
  }
  return out;
}

}  // namespace rt::test
