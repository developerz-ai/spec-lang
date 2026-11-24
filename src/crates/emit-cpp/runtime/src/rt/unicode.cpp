// UTF-8 validity, code points and the 25 White_Space code points of docs/contract.md section 1.
#include "rt/unicode.hpp"

#include <bit>
#include <cstdint>
#include <cstring>

namespace rt::unicode {
namespace {

constexpr std::uint64_t high_bits = 0x8080808080808080ULL;

// Length of the leading ASCII run, eight bytes at a time.
std::size_t ascii_prefix(std::string_view s) noexcept {
  std::size_t i = 0;
  while (s.size() - i >= 8) {
    std::uint64_t word = 0;
    std::memcpy(&word, s.data() + i, 8);
    if ((word & high_bits) != 0) {
      break;
    }
    i += 8;
  }
  while (i < s.size() && static_cast<unsigned char>(s[i]) < 0x80) {
    ++i;
  }
  return i;
}

struct Decoded {
  char32_t value;
  std::size_t length;  // 0 when malformed
};

Decoded decode_one(std::string_view s, std::size_t i) noexcept {
  const auto b0 = static_cast<unsigned char>(s[i]);
  if (b0 < 0x80) {
    return {b0, 1};
  }
  std::size_t length = 0;
  char32_t value = 0;
  char32_t smallest = 0;
  if ((b0 & 0xE0U) == 0xC0U) {
    length = 2;
    value = b0 & 0x1FU;
    smallest = 0x80;
  } else if ((b0 & 0xF0U) == 0xE0U) {
    length = 3;
    value = b0 & 0x0FU;
    smallest = 0x800;
  } else if ((b0 & 0xF8U) == 0xF0U) {
    length = 4;
    value = b0 & 0x07U;
    smallest = 0x10000;
  } else {
    return {0, 0};
  }
  if (s.size() - i < length) {
    return {0, 0};
  }
  for (std::size_t k = 1; k < length; ++k) {
    const auto b = static_cast<unsigned char>(s[i + k]);
    if ((b & 0xC0U) != 0x80U) {
      return {0, 0};
    }
    value = (value << 6U) | (b & 0x3FU);
  }
  if (value < smallest || value > 0x10FFFF || (value >= 0xD800 && value <= 0xDFFF)) {
    return {0, 0};
  }
  return {value, length};
}

}  // namespace

bool is_valid_utf8(std::string_view bytes) noexcept {
  std::size_t i = 0;
  while (i < bytes.size()) {
    i += ascii_prefix(bytes.substr(i));
    if (i == bytes.size()) {
      break;
    }
    const Decoded d = decode_one(bytes, i);
    if (d.length == 0) {
      return false;
    }
    i += d.length;
  }
  return true;
}

std::size_t code_points(std::string_view text) noexcept {
  // Every byte but a continuation byte (10xxxxxx) starts a code point; eight bytes at a time.
  constexpr std::uint64_t low7 = 0x7F7F7F7F7F7F7F7FULL;
  std::size_t count = 0;
  std::size_t i = 0;
  for (; text.size() - i >= 8; i += 8) {
    std::uint64_t word = 0;
    std::memcpy(&word, text.data() + i, 8);
    // A continuation byte has bit 7 set and bit 6 clear.
    const std::uint64_t continuation = word & ~(word << 1U) & ~low7;
    count += 8 - static_cast<std::size_t>(std::popcount(continuation));
  }
  for (; i < text.size(); ++i) {
    count += (static_cast<unsigned char>(text[i]) & 0xC0U) != 0x80U ? 1 : 0;
  }
  return count;
}

bool is_blank(std::string_view text) noexcept {
  std::size_t i = 0;
  while (i < text.size()) {
    const Decoded d = decode_one(text, i);
    if (d.length == 0 || !is_white_space(d.value)) {
      return false;
    }
    i += d.length;
  }
  return true;
}

void append_utf8(std::string& out, char32_t c) {
  if (c < 0x80) {
    out.push_back(static_cast<char>(c));
  } else if (c < 0x800) {
    out.push_back(static_cast<char>(0xC0U | (c >> 6U)));
    out.push_back(static_cast<char>(0x80U | (c & 0x3FU)));
  } else if (c < 0x10000) {
    out.push_back(static_cast<char>(0xE0U | (c >> 12U)));
    out.push_back(static_cast<char>(0x80U | ((c >> 6U) & 0x3FU)));
    out.push_back(static_cast<char>(0x80U | (c & 0x3FU)));
  } else {
    out.push_back(static_cast<char>(0xF0U | (c >> 18U)));
    out.push_back(static_cast<char>(0x80U | ((c >> 12U) & 0x3FU)));
    out.push_back(static_cast<char>(0x80U | ((c >> 6U) & 0x3FU)));
    out.push_back(static_cast<char>(0x80U | (c & 0x3FU)));
  }
}

}  // namespace rt::unicode
