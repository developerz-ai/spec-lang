// The `cli` library's runtime for C++ (libs/cli/real/cpp): a strict JSON reader for the store.
#include "lib/cli/json.hpp"

#include <algorithm>

#include "rt/halt.hpp"
#include "rt/text.hpp"

namespace lib::cli {

namespace {

// Deeper than any store; it bounds recursion so hostile input cannot overflow the stack.
constexpr int kMaxDepth = 64;

struct Malformed {};

class Reader {
 public:
  explicit Reader(std::string_view text) : text_(text) {}

  Json document() {
    space();
    Json value = this->value(0);
    space();
    if (at_ != text_.size()) throw Malformed{};
    return value;
  }

 private:
  char peek() const {
    return at_ < text_.size() ? text_[at_] : '\0';
  }
  bool done() const {
    return at_ >= text_.size();
  }

  void space() {
    while (!done() && (peek() == ' ' || peek() == '\t' || peek() == '\n' || peek() == '\r')) ++at_;
  }

  void expect(char c) {
    if (done() || peek() != c) throw Malformed{};
    ++at_;
  }

  void word(std::string_view w) {
    if (!text_.substr(at_).starts_with(w)) throw Malformed{};
    at_ += w.size();
  }

  Json value(int depth) {
    if (depth > kMaxDepth || done()) throw Malformed{};
    switch (peek()) {
      case '{':
        return object(depth);
      case '[':
        return array(depth);
      case '"': {
        Json j{.kind = Json::Kind::String};
        j.string = string();
        return j;
      }
      case 't':
        word("true");
        return Json{.kind = Json::Kind::Bool, .boolean = true};
      case 'f':
        word("false");
        return Json{.kind = Json::Kind::Bool, .boolean = false};
      case 'n':
        word("null");
        return Json{};
      default:
        return number();
    }
  }

  Json object(int depth) {
    expect('{');
    Json j{.kind = Json::Kind::Object};
    space();
    if (peek() == '}') {
      ++at_;
      return j;
    }
    for (;;) {
      space();
      std::string key = string();
      if (j.get(key) != nullptr) throw Malformed{};
      space();
      expect(':');
      space();
      Json item = value(depth + 1);
      j.entries.emplace_back(std::move(key), std::move(item));
      space();
      if (peek() == '}') {
        ++at_;
        return j;
      }
      expect(',');
    }
  }

  Json array(int depth) {
    expect('[');
    Json j{.kind = Json::Kind::Array};
    space();
    if (peek() == ']') {
      ++at_;
      return j;
    }
    for (;;) {
      space();
      j.items.push_back(value(depth + 1));
      space();
      if (peek() == ']') {
        ++at_;
        return j;
      }
      expect(',');
    }
  }

  static bool digit(char c) {
    return c >= '0' && c <= '9';
  }

  void digits() {
    if (!digit(peek())) throw Malformed{};
    while (digit(peek())) ++at_;
  }

  Json number() {
    const std::size_t from = at_;
    if (peek() == '-') ++at_;
    if (peek() == '0') {
      ++at_;
    } else {
      digits();
    }
    bool whole = true;
    if (peek() == '.') {
      ++at_;
      digits();
      whole = false;
    }
    if (peek() == 'e' || peek() == 'E') {
      ++at_;
      if (peek() == '+' || peek() == '-') ++at_;
      digits();
      whole = false;
    }
    Json j{.kind = Json::Kind::Number};
    if (!whole) return j;
    // Exact, or absent when it is past what an Int holds: a reader of the store refuses it either way.
    try {
      const std::string_view literal = text_.substr(from, at_ - from);
      const bool negative = literal.starts_with('-');
      rt::Int n;
      for (const char c : literal.substr(negative ? 1 : 0)) n = n * rt::Int{10} - rt::Int{c - '0'};
      j.integer = negative ? n : -n;
    } catch (const rt::Stop&) {
    }
    return j;
  }

  char32_t hex4() {
    char32_t out = 0;
    for (int k = 0; k < 4; ++k) {
      const char c = peek();
      char32_t d = 0;
      if (c >= '0' && c <= '9') {
        d = static_cast<char32_t>(c - '0');
      } else if (c >= 'a' && c <= 'f') {
        d = static_cast<char32_t>(c - 'a' + 10);
      } else if (c >= 'A' && c <= 'F') {
        d = static_cast<char32_t>(c - 'A' + 10);
      } else {
        throw Malformed{};
      }
      out = (out << 4U) | d;
      ++at_;
    }
    return out;
  }

  // `\uXXXX`, joining a surrogate pair; a lone surrogate is not text.
  char32_t unicode_escape() {
    const char32_t u = hex4();
    if (u >= 0xDC00 && u <= 0xDFFF) throw Malformed{};
    if (u < 0xD800 || u > 0xDBFF) return u;
    expect('\\');
    expect('u');
    const char32_t low = hex4();
    if (low < 0xDC00 || low > 0xDFFF) throw Malformed{};
    return 0x10000 + ((u - 0xD800) << 10U) + (low - 0xDC00);
  }

  std::string string() {
    expect('"');
    std::string out;
    for (;;) {
      if (done()) throw Malformed{};
      const char c = text_[at_++];
      if (c == '"') return out;
      if (static_cast<unsigned char>(c) < 0x20) throw Malformed{};
      if (c != '\\') {
        out.push_back(c);
        continue;
      }
      if (done()) throw Malformed{};
      switch (text_[at_++]) {
        case '"':
          out.push_back('"');
          break;
        case '\\':
          out.push_back('\\');
          break;
        case '/':
          out.push_back('/');
          break;
        case 'b':
          out.push_back('\b');
          break;
        case 'f':
          out.push_back('\f');
          break;
        case 'n':
          out.push_back('\n');
          break;
        case 'r':
          out.push_back('\r');
          break;
        case 't':
          out.push_back('\t');
          break;
        case 'u':
          rt::unicode::append(out, unicode_escape());
          break;
        default:
          throw Malformed{};
      }
    }
  }

  std::string_view text_;
  std::size_t at_ = 0;
};

}  // namespace

const Json* Json::get(std::string_view key) const {
  const auto found = std::ranges::find_if(entries, [&](const auto& e) { return e.first == key; });
  return found == entries.end() ? nullptr : &found->second;
}

std::optional<Json> read_json(std::string_view text) {
  try {
    return Reader{text}.document();
  } catch (const Malformed&) {
    return std::nullopt;
  }
}

}  // namespace lib::cli
