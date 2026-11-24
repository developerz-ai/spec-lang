// The C++ printer's runtime, the same in every project `spec-lang build` prints. Do not edit.
// Text by code point over UTF-8.
#include "rt/text.hpp"

namespace rt {

namespace unicode {

char32_t next(std::string_view text, std::size_t& at) {
  const auto byte = [&](std::size_t i) {
    return static_cast<unsigned char>(i < text.size() ? text[i] : 0);
  };
  const unsigned char lead = byte(at);
  if (lead < 0x80) {
    at += 1;
    return lead;
  }
  if (lead < 0xE0) {
    const char32_t c = (static_cast<char32_t>(lead & 0x1F) << 6) | (byte(at + 1) & 0x3F);
    at += 2;
    return c;
  }
  if (lead < 0xF0) {
    const char32_t c = (static_cast<char32_t>(lead & 0x0F) << 12) | (static_cast<char32_t>(byte(at + 1) & 0x3F) << 6) |
                       (byte(at + 2) & 0x3F);
    at += 3;
    return c;
  }
  const char32_t c = (static_cast<char32_t>(lead & 0x07) << 18) | (static_cast<char32_t>(byte(at + 1) & 0x3F) << 12) |
                     (static_cast<char32_t>(byte(at + 2) & 0x3F) << 6) | (byte(at + 3) & 0x3F);
  at += 4;
  return c;
}

void append(Text& out, char32_t c) {
  if (c < 0x80) {
    out.push_back(static_cast<char>(c));
  } else if (c < 0x800) {
    out.push_back(static_cast<char>(0xC0 | (c >> 6)));
    out.push_back(static_cast<char>(0x80 | (c & 0x3F)));
  } else if (c < 0x10000) {
    out.push_back(static_cast<char>(0xE0 | (c >> 12)));
    out.push_back(static_cast<char>(0x80 | ((c >> 6) & 0x3F)));
    out.push_back(static_cast<char>(0x80 | (c & 0x3F)));
  } else {
    out.push_back(static_cast<char>(0xF0 | (c >> 18)));
    out.push_back(static_cast<char>(0x80 | ((c >> 12) & 0x3F)));
    out.push_back(static_cast<char>(0x80 | ((c >> 6) & 0x3F)));
    out.push_back(static_cast<char>(0x80 | (c & 0x3F)));
  }
}

// Unicode White_Space: exactly 25 code points. Not U+200B, U+FEFF, U+180E.
bool is_white_space(char32_t c) {
  return (c >= 0x09 && c <= 0x0D) || c == 0x20 || c == 0x85 || c == 0xA0 || c == 0x1680 ||
         (c >= 0x2000 && c <= 0x200A) || c == 0x2028 || c == 0x2029 || c == 0x202F || c == 0x205F || c == 0x3000;
}

bool is_control(char32_t c) {
  return c < 0x20 || (c >= 0x7F && c <= 0x9F);
}

}  // namespace unicode

namespace {
// The byte offset after `count` code points, or the size when there are fewer.
std::size_t offset_of(const Text& text, Int count) {
  std::size_t at = 0;
  for (Int seen; seen < count && at < text.size(); seen = seen + Int{1}) unicode::next(text, at);
  return at;
}
}  // namespace

Int size(const Text& text) {
  std::size_t count = 0;
  for (const char byte : text) {
    if ((static_cast<unsigned char>(byte) & 0xC0) != 0x80) ++count;
  }
  return Int::of_size(count);
}

List<Int> bytes(const Text& text) {
  List<Int> out;
  out.reserve(text.size());
  for (const char byte : text) out.emplace_back(static_cast<unsigned char>(byte));
  return out;
}

Text trim(const Text& text) {
  std::size_t from = 0;
  while (from < text.size()) {
    std::size_t after = from;
    if (!unicode::is_white_space(unicode::next(text, after))) break;
    from = after;
  }
  std::size_t to = from;
  for (std::size_t at = from; at < text.size();) {
    if (!unicode::is_white_space(unicode::next(text, at))) to = at;
  }
  return text.substr(from, to - from);
}

bool starts_with(const Text& text, const Text& prefix) {
  return text.starts_with(prefix);
}
bool contains(const Text& text, const Text& part) {
  return text.find(part) != Text::npos;
}

List<Text> split(const Text& text, const Text& separator) {
  if (separator.empty()) stop("E-TYPE-ARITY", "`text.split` takes one non-empty Text");
  List<Text> out;
  std::size_t from = 0;
  for (std::size_t hit = text.find(separator); hit != Text::npos; hit = text.find(separator, from)) {
    out.push_back(text.substr(from, hit - from));
    from = hit + separator.size();
  }
  out.push_back(text.substr(from));
  return out;
}

Text replace(const Text& text, const Text& old, const Text& with) {
  if (old.empty()) stop("E-TYPE-ARITY", "`text.replace` takes two Text, the first not empty");
  Text out;
  std::size_t from = 0;
  for (std::size_t hit = text.find(old); hit != Text::npos; hit = text.find(old, from)) {
    out.append(text, from, hit - from);
    out += with;
    from = hit + old.size();
  }
  out.append(text, from);
  return out;
}

Text take(const Text& text, Int count) {
  return text.substr(0, offset_of(text, count));
}
Text drop(const Text& text, Int count) {
  return text.substr(offset_of(text, count));
}

Text join(const List<Text>& parts, const Text& separator) {
  Text out;
  for (std::size_t i = 0; i < parts.size(); ++i) {
    if (i > 0) out += separator;
    out += parts[i];
  }
  return out;
}

Text concat(std::initializer_list<Text> parts) {
  Text out;
  for (const auto& part : parts) out += part;
  return out;
}

}  // namespace rt
