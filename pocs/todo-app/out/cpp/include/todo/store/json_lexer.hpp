// Projected by hand from "Accepted on read" (docs/contract.md section 3) for @store (src/apps/cpp.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <string_view>

namespace todo::store::json {

// A cursor over one buffer of JSON text that is already known to be valid UTF-8. Every method
// returns false or nullopt on text that is not JSON; the caller then rejects the whole document.
class Lexer {
 public:
  explicit Lexer(std::string_view text) noexcept : text_(text) {}

  void skip_space() noexcept;
  [[nodiscard]] bool at_end() noexcept;
  // Skips white space, then consumes c if it is next.
  [[nodiscard]] bool eat(char c) noexcept;
  [[nodiscard]] bool peek_is(char c) noexcept;

  // A string. The view points into the buffer when the string has no escape, else into scratch.
  [[nodiscard]] std::optional<std::string_view> string(std::string& scratch);

  // A JSON number that is an integer (no fraction, no exponent), not negative, and at most max.
  // Any other number, or any other value, is nullopt. Never rounds.
  [[nodiscard]] std::optional<std::uint64_t> integer(std::uint64_t max) noexcept;

 private:
  [[nodiscard]] bool hex4(char32_t& out) noexcept;
  std::string_view text_;
  std::size_t at_ = 0;
};

}  // namespace todo::store::json
