// The `cli` library's runtime for C++ (libs/cli/real/cpp): a program its user runs from a shell.
// One command per rule with trigger `cli("name")`, its parameters the arguments, the world kept in the file
// `@store(file(path))` names between runs, `out.line` on standard output after the rule commits.
#pragma once

#include <string>
#include <string_view>
#include <vector>

#include "rt/meta.hpp"

namespace lib::cli {

// What one run gave: both streams as bytes, and the exit code.
struct Output {
  std::string out;
  std::string err;
  int code = 0;
};

// Exit codes: a rule refused, a usage error, the store.
inline constexpr int kRefused = 1;
inline constexpr int kUsage = 2;
inline constexpr int kStore = 3;

// One whole run. `args` excludes the program's name; `store_override` replaces the store's path when not empty.
Output run(const rt::meta::App& app, const std::vector<std::string_view>& args, const std::string& store_override);

// The process: argv and the environment in, the two streams and an exit code out.
int main(const rt::meta::App& app, int argc, char** argv);

}  // namespace lib::cli
