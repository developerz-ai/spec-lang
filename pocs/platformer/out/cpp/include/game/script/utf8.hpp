// Projected by hand from "Invalid UTF-8 counts as an unknown key at its line" and `unknown key "c"` (docs/contract.md section 2): Text is Unicode, so a key is a code point, not a byte. Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <cstddef>
#include <string_view>

namespace game::script {

// The length in bytes of the well-formed UTF-8 sequence text starts with (1 to 4), or 0 when it
// starts with bytes that are not one: a stray continuation, an overlong form, a surrogate, a
// value above U+10FFFF or a sequence cut short.
constexpr std::size_t utf8_length(std::string_view text) {
  if (text.empty()) {
    return 0;
  }
  const auto byte = [&](std::size_t i) { return static_cast<unsigned char>(text[i]); };
  const auto tail = [&](std::size_t i) { return i < text.size() && (byte(i) & 0xC0) == 0x80; };
  const unsigned char b0 = byte(0);
  if (b0 < 0x80) {
    return 1;
  }
  if (b0 >= 0xC2 && b0 <= 0xDF) {
    return tail(1) ? 2 : 0;
  }
  if (b0 >= 0xE0 && b0 <= 0xEF) {
    if (!tail(1) || !tail(2)) {
      return 0;
    }
    if ((b0 == 0xE0 && byte(1) < 0xA0) || (b0 == 0xED && byte(1) > 0x9F)) {
      return 0;
    }
    return 3;
  }
  if (b0 >= 0xF0 && b0 <= 0xF4) {
    if (!tail(1) || !tail(2) || !tail(3)) {
      return 0;
    }
    if ((b0 == 0xF0 && byte(1) < 0x90) || (b0 == 0xF4 && byte(1) > 0x8F)) {
      return 0;
    }
    return 4;
  }
  return 0;
}

constexpr bool is_utf8(std::string_view text) {
  while (!text.empty()) {
    const std::size_t n = utf8_length(text);
    if (n == 0) {
      return false;
    }
    text.remove_prefix(n);
  }
  return true;
}

}  // namespace game::script
