// Projected by hand from fn is_title (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "todo/model/text.hpp"

#include "todo/model/unicode.hpp"

namespace todo::model {
namespace {

constexpr std::size_t max_title = 200;

// The line breaks above U+007F that a title refuses. In valid UTF-8 a code point's bytes match only at
// that code point, so a byte search is t.contains.
constexpr bool has_line_break(std::string_view t) noexcept {
  return t.contains("\u0085") || t.contains("\u2028") || t.contains("\u2029");
}

// The ASCII fast path: one pass decides bytes, blankness and size together. The refused line breaks
// are not ASCII, so only the other path looks for them.
struct AsciiScan {
  bool all_ascii = true;
  bool bytes_ok = true;
  bool blank = true;
};

AsciiScan scan(std::string_view t) noexcept {
  AsciiScan r;
  for (const char c : t) {
    const auto b = static_cast<unsigned char>(c);
    if (b >= 0x80) {
      r.all_ascii = false;
      continue;
    }
    if (b < 32 || b == 127) {
      r.bytes_ok = false;
    }
    if (!unicode::is_white_space(b)) {
      r.blank = false;
    }
  }
  return r;
}

}  // namespace

bool is_title(std::string_view t) noexcept {
  const AsciiScan s = scan(t);
  if (!s.bytes_ok) {
    return false;
  }
  if (s.all_ascii) {
    return !s.blank && t.size() <= max_title;
  }
  if (t.size() > 4 * max_title || !unicode::is_valid_utf8(t)) {
    return false;
  }
  return !has_line_break(t) && !unicode::is_blank(t) && unicode::code_points(t) <= max_title;
}

}  // namespace todo::model
