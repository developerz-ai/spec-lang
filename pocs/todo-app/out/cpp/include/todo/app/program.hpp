// Projected by hand from app TodoCpp (src/apps/cpp.slg) with the cli vocabulary (docs/cli-vocabulary.md). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <span>
#include <string>
#include <string_view>

namespace todo::app {

struct RunResult {
  int code = 0;
  std::string out;  // standard output, already committed: print after the lock is gone
  std::string err;  // standard error
};

// One run of the binary: parse, lock, load, rule, store, lines. args excludes the program name.
[[nodiscard]] RunResult run(std::span<const std::string_view> args, const std::string& store_path);

}  // namespace todo::app
