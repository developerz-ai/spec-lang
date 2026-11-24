// Projected by hand from fn is_title, fn is_tag (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <string_view>

namespace todo::model {

// t.trim().size >= 1 and t.size <= 200 and t.bytes.all(it >= 32 and it != 127)
// and not t.contains("\u{0085}") and not t.contains("\u{2028}") and not t.contains("\u{2029}").
// Text is Unicode, so bytes that are not UTF-8 are not a Text and not a Title.
[[nodiscard]] bool is_title(std::string_view t) noexcept;

// 1 <= t.size <= 32 and t.bytes.all(97 <= it <= 122 or 48 <= it <= 57 or it == 45) and not t.starts_with("-").
[[nodiscard]] constexpr bool is_tag(std::string_view t) noexcept {
  // Every allowed byte is ASCII, so bytes == code points.
  if (t.empty() || t.size() > 32 || t.front() == '-') {
    return false;
  }
  for (const char c : t) {
    const bool ok = (c >= 'a' && c <= 'z') || (c >= '0' && c <= '9') || c == '-';
    if (!ok) {
      return false;
    }
  }
  return true;
}

}  // namespace todo::model
