// Projected by hand from the test forms of the language card (docs/idea/01-language-card.md, Tests) for hop (src/hop/tests/moves.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "harness.hpp"

#include <algorithm>
#include <cstdio>
#include <exception>
#include <vector>

namespace harness {
namespace {

struct Entry {
  std::string_view name;
  TestFn fn;
};

struct Failed {
  std::string message;
};

struct Skipped {
  std::string why;
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

void skip(std::string_view why) { throw Skipped{std::string(why)}; }

std::string show(std::string_view s) {
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

}  // namespace harness

// Runs every registered test, or only those named on the command line. Exit 0: all passed;
// 1: one failed or none ran; 77: none failed and at least one was skipped.
int main(int argc, char** argv) {
  const std::vector<std::string_view> only(argv + 1, argv + argc);
  int failed = 0;
  int skipped = 0;
  int ran = 0;
  for (const auto& [name, fn] : harness::registry()) {
    if (!only.empty() && std::ranges::find(only, name) == only.end()) {
      continue;
    }
    ++ran;
    const int size = static_cast<int>(name.size());
    try {
      fn();
      std::printf("ok   %.*s\n", size, name.data());
    } catch (const harness::Failed& f) {
      ++failed;
      std::printf("FAIL %.*s\n  %s\n", size, name.data(), f.message.c_str());
    } catch (const harness::Skipped& s) {
      ++skipped;
      std::printf("skip %.*s: %s\n", size, name.data(), s.why.c_str());
    } catch (const std::exception& e) {
      ++failed;
      std::printf("FAIL %.*s\n  exception: %s\n", size, name.data(), e.what());
    }
  }
  std::printf("%d of %d passed, %d skipped\n", ran - failed - skipped, ran, skipped);
  if (failed > 0 || ran == 0) {
    return 1;
  }
  return skipped > 0 ? 77 : 0;
}
