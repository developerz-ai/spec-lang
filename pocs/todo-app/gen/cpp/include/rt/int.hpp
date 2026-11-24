// The C++ printer's runtime, the same in every project `spec-lang build` prints. Do not edit.
// The exact Int of spec-lang, as a checked 128-bit integer: every operation whose exact result leaves
// -2^127 .. 2^127 - 1 stops with E-RUNTIME instead of wrapping.
#pragma once

#include <compare>
#include <string>

#include "rt/halt.hpp"

namespace rt {

__extension__ using int128 = __int128;
__extension__ using uint128 = unsigned __int128;

class Int {
 public:
  constexpr Int() = default;
  constexpr Int(long long value) : value_(value) {}  // NOLINT: whole numbers are Ints
  static constexpr Int wide(int128 value) {
    Int n;
    n.value_ = value;
    return n;
  }
  static constexpr Int of_size(std::size_t n) {
    return wide(static_cast<int128>(n));
  }
  static constexpr Int min() {
    return wide(static_cast<int128>(static_cast<uint128>(1) << 127));
  }

  constexpr int128 value() const {
    return value_;
  }
  // The value as an index into a collection of `size` items, or `size` when it is outside.
  constexpr std::size_t index(std::size_t size) const {
    return value_ >= 0 && value_ < static_cast<int128>(size) ? static_cast<std::size_t>(value_) : size;
  }
  constexpr bool fits_long() const {
    return value_ >= -(int128{1} << 63) && value_ < (int128{1} << 63);
  }
  constexpr long long to_long() const {
    return static_cast<long long>(value_);
  }

  friend constexpr Int operator+(Int a, Int b) {
    int128 r = 0;
    if (__builtin_add_overflow(a.value_, b.value_, &r)) overflow();
    return wide(r);
  }
  friend constexpr Int operator-(Int a, Int b) {
    int128 r = 0;
    if (__builtin_sub_overflow(a.value_, b.value_, &r)) overflow();
    return wide(r);
  }
  friend constexpr Int operator*(Int a, Int b) {
    int128 r = 0;
    if (__builtin_mul_overflow(a.value_, b.value_, &r)) overflow();
    return wide(r);
  }
  friend constexpr Int operator-(Int a) {
    return Int{} - a;
  }
  friend constexpr std::strong_ordering operator<=>(Int a, Int b) = default;
  friend constexpr bool operator==(Int a, Int b) = default;

 private:
  [[noreturn]] static void overflow() {
    throw Stop{"E-RUNTIME", "an Int beyond the 128-bit range of this runtime (the language's Int is unbounded)"};
  }
  int128 value_ = 0;
};

inline namespace literals {
// `170141183460469231731687303715884105727_i`: an Int literal of any length that fits.
constexpr Int operator""_i(const char* digits) {
  Int n;
  for (const char* p = digits; *p != '\0'; ++p) {
    if (*p >= '0' && *p <= '9') n = n * Int{10} + Int{*p - '0'};
  }
  return n;
}
}  // namespace literals

// Floored modulo: the result has the sign of the divisor. A zero divisor fails DivByZero.
Int mod(Int a, Int b);
// Exact power; a negative exponent stops E-TYPE-MISMATCH.
Int pow(Int base, Int exponent);
// `/` on Int is an open choice of the language: always stops E-OPEN.
[[noreturn]] Int div(Int a, Int b);
Int band(Int a, Int b);
Int bor(Int a, Int b);
Int bxor(Int a, Int b);
Int shl(Int a, Int b);
Int shr(Int a, Int b);
Int abs(Int a);
constexpr Int min(Int a, Int b) {
  return b < a ? b : a;
}
constexpr Int max(Int a, Int b) {
  return b > a ? b : a;
}
Int clamp(Int value, Int lo, Int hi);
std::string to_string(Int n);

}  // namespace rt
