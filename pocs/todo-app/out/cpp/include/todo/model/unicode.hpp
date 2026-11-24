// Projected by hand from Text.size and Text.trim as fn is_title uses them (src/todo/model.slg; docs/contract.md section 1). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <cstddef>
#include <string>
#include <string_view>

namespace todo::model::unicode {

// True when the bytes are well-formed UTF-8: no overlong form, no surrogate, nothing above U+10FFFF.
[[nodiscard]] bool is_valid_utf8(std::string_view bytes) noexcept;

// Text.size: code points. Precondition: is_valid_utf8(text).
[[nodiscard]] std::size_t code_points(std::string_view text) noexcept;

// One of the 25 Unicode White_Space code points the contract lists, and nothing else.
[[nodiscard]] constexpr bool is_white_space(char32_t c) noexcept {
  return (c >= 0x09 && c <= 0x0D) || c == 0x20 || c == 0x85 || c == 0xA0 || c == 0x1680 ||
         (c >= 0x2000 && c <= 0x200A) || c == 0x2028 || c == 0x2029 || c == 0x202F || c == 0x205F ||
         c == 0x3000;
}

// text.trim().size == 0. Precondition: is_valid_utf8(text).
[[nodiscard]] bool is_blank(std::string_view text) noexcept;

// Appends the UTF-8 form of a scalar value (not a surrogate, at most U+10FFFF).
void append_utf8(std::string& out, char32_t c);

}  // namespace todo::model::unicode
