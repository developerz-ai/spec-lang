// Projected by hand from the test forms of the language card (docs/idea/01-language-card.md, Tests) for todo (src/todo/tests/scenarios.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <concepts>
#include <optional>
#include <source_location>
#include <string>
#include <string_view>
#include <vector>

namespace harness {

using TestFn = void (*)();

struct Registration {
  Registration(std::string_view name, TestFn fn);
};

// Ends the current test as failed.
[[noreturn]] void fail(std::string_view what, std::source_location where = std::source_location::current());

inline void check(bool ok, std::string_view expr,
                  std::source_location where = std::source_location::current()) {
  if (!ok) {
    fail(expr, where);
  }
}

inline std::string show(std::string_view s) {
  std::string out = "\"";
  for (const char c : s) {
    if (c == '\n') {
      out += "\\n";
    } else if (c == '"' || c == '\\') {
      out += '\\';
      out += c;
    } else {
      out += c;
    }
  }
  return out + "\"";
}
inline std::string show(const std::string& s) { return show(std::string_view(s)); }
inline std::string show(const char* s) { return show(std::string_view(s)); }
inline std::string show(bool b) { return b ? "true" : "false"; }
template <std::integral T>
std::string show(T v) {
  return std::to_string(v);
}
template <class T>
  requires(!std::integral<T>)
std::string show(const T&) {
  return "<value>";
}

template <class A, class B>
void check_eq(const A& actual, const B& expected, std::string_view expr,
              std::source_location where = std::source_location::current()) {
  if (!(actual == expected)) {
    fail(std::string(expr) + "\n    actual:   " + show(actual) + "\n    expected: " + show(expected), where);
  }
}

}  // namespace harness

#define TODO_CAT2(a, b) a##b
#define TODO_CAT(a, b) TODO_CAT2(a, b)
#define TEST(name)                                                                  \
  static void name();                                                               \
  static const ::harness::Registration TODO_CAT(name, _registration){#name, &name}; \
  static void name()
#define CHECK(expr) ::harness::check(static_cast<bool>(expr), #expr)
#define CHECK_EQ(actual, expected) ::harness::check_eq((actual), (expected), #actual " == " #expected)
