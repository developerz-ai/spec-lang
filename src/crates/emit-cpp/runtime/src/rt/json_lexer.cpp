// The strict JSON lexer under the generated, schema-directed store reader (docs/contract.md section 3).
#include "rt/json_lexer.hpp"

#include "rt/unicode.hpp"

namespace rt::json {
namespace {

constexpr bool is_digit(char c) noexcept { return c >= '0' && c <= '9'; }

}  // namespace

void Lexer::skip_space() noexcept {
  while (at_ < text_.size()) {
    const char c = text_[at_];
    if (c != ' ' && c != '\t' && c != '\n' && c != '\r') {
      return;
    }
    ++at_;
  }
}

bool Lexer::at_end() noexcept {
  skip_space();
  return at_ == text_.size();
}

bool Lexer::peek_is(char c) noexcept {
  skip_space();
  return at_ < text_.size() && text_[at_] == c;
}

bool Lexer::eat(char c) noexcept {
  if (!peek_is(c)) {
    return false;
  }
  ++at_;
  return true;
}

bool Lexer::hex4(char32_t& out) noexcept {
  if (text_.size() - at_ < 4) {
    return false;
  }
  out = 0;
  for (int k = 0; k < 4; ++k) {
    const char c = text_[at_++];
    unsigned digit = 0;
    if (c >= '0' && c <= '9') {
      digit = static_cast<unsigned>(c - '0');
    } else if (c >= 'a' && c <= 'f') {
      digit = static_cast<unsigned>(c - 'a' + 10);
    } else if (c >= 'A' && c <= 'F') {
      digit = static_cast<unsigned>(c - 'A' + 10);
    } else {
      return false;
    }
    out = (out << 4U) | digit;
  }
  return true;
}

std::optional<std::string_view> Lexer::string(std::string& scratch) {
  if (!eat('"')) {
    return std::nullopt;
  }
  const std::size_t start = at_;
  // Fast path: no escape, so the string is a view into the buffer.
  while (at_ < text_.size()) {
    const auto c = static_cast<unsigned char>(text_[at_]);
    if (c == '"') {
      return text_.substr(start, at_++ - start);
    }
    if (c == '\\') {
      break;
    }
    if (c < 0x20) {
      return std::nullopt;
    }
    ++at_;
  }
  scratch.assign(text_.substr(start, at_ - start));
  while (at_ < text_.size()) {
    const auto c = static_cast<unsigned char>(text_[at_++]);
    if (c == '"') {
      return std::string_view(scratch);
    }
    if (c < 0x20) {
      return std::nullopt;
    }
    if (c != '\\') {
      scratch.push_back(static_cast<char>(c));
      continue;
    }
    if (at_ == text_.size()) {
      return std::nullopt;
    }
    switch (text_[at_++]) {
      case '"':
        scratch.push_back('"');
        break;
      case '\\':
        scratch.push_back('\\');
        break;
      case '/':
        scratch.push_back('/');
        break;
      case 'b':
        scratch.push_back('\b');
        break;
      case 'f':
        scratch.push_back('\f');
        break;
      case 'n':
        scratch.push_back('\n');
        break;
      case 'r':
        scratch.push_back('\r');
        break;
      case 't':
        scratch.push_back('\t');
        break;
      case 'u': {
        char32_t unit = 0;
        if (!hex4(unit)) {
          return std::nullopt;
        }
        if (unit >= 0xDC00 && unit <= 0xDFFF) {
          return std::nullopt;  // a low surrogate with no high one before it
        }
        if (unit >= 0xD800 && unit <= 0xDBFF) {
          char32_t low = 0;
          if (text_.substr(at_, 2) != "\\u") {
            return std::nullopt;  // a high surrogate alone
          }
          at_ += 2;
          if (!hex4(low) || low < 0xDC00 || low > 0xDFFF) {
            return std::nullopt;
          }
          unit = 0x10000 + ((unit - 0xD800) << 10U) + (low - 0xDC00);
        }
        unicode::append_utf8(scratch, unit);
        break;
      }
      default:
        return std::nullopt;
    }
  }
  return std::nullopt;
}

std::optional<std::uint64_t> Lexer::integer(std::uint64_t max) noexcept {
  skip_space();
  const std::size_t start = at_;
  if (at_ < text_.size() && text_[at_] == '-') {
    ++at_;
  }
  const std::size_t digits_at = at_;
  while (at_ < text_.size() && is_digit(text_[at_])) {
    ++at_;
  }
  const std::size_t digits = at_ - digits_at;
  if (digits == 0 || (digits > 1 && text_[digits_at] == '0')) {
    return std::nullopt;  // not a JSON number
  }
  if (at_ < text_.size() && (text_[at_] == '.' || text_[at_] == 'e' || text_[at_] == 'E')) {
    return std::nullopt;  // a JSON number, but not an integer
  }
  if (digits_at != start) {
    return std::nullopt;  // negative, or -0: below every range the store uses
  }
  if (digits > 19) {
    return std::nullopt;  // above 2^63, so above any max a caller passes
  }
  std::uint64_t value = 0;
  for (std::size_t k = digits_at; k < at_; ++k) {
    value = value * 10 + static_cast<std::uint64_t>(text_[k] - '0');
  }
  if (value > max) {
    return std::nullopt;
  }
  return value;
}

}  // namespace rt::json
