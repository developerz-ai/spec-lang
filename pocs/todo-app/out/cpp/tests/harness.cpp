// Projected by hand from the test forms of the language card (docs/idea/01-language-card.md, Tests) for todo (src/todo/tests/scenarios.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "harness.hpp"

#include <algorithm>
#include <cstdio>
#include <exception>
#include <utility>

namespace harness {
namespace {

struct Entry {
  std::string_view name;
  TestFn fn;
};

struct Failed {
  std::string message;
};

std::vector<Entry>& registry() {
  static std::vector<Entry> entries;
  return entries;
}

}  // namespace

Registration::Registration(std::string_view name, TestFn fn) { registry().push_back({name, fn}); }

void fail(std::string_view what, std::source_location where) {
  throw Failed{std::string(where.file_name()) + ":" + std::to_string(where.line()) + ": " +
               std::string(what)};
}

}  // namespace harness

// Runs every registered test, or only those named on the command line.
int main(int argc, char** argv) {
  const std::vector<std::string_view> only(argv + 1, argv + argc);
  int failed = 0;
  int ran = 0;
  for (const auto& [name, fn] : harness::registry()) {
    if (!only.empty() && std::ranges::find(only, name) == only.end()) {
      continue;
    }
    ++ran;
    try {
      fn();
      std::printf("ok   %.*s\n", static_cast<int>(name.size()), name.data());
    } catch (const harness::Failed& f) {
      ++failed;
      std::printf("FAIL %.*s\n  %s\n", static_cast<int>(name.size()), name.data(), f.message.c_str());
    } catch (const std::exception& e) {
      ++failed;
      std::printf("FAIL %.*s\n  exception: %s\n", static_cast<int>(name.size()), name.data(), e.what());
    }
  }
  std::printf("%d of %d passed\n", ran - failed, ran);
  return failed == 0 && ran > 0 ? 0 : 1;
}
