// The `game` library's runtime for C++ (libs/game/real/cpp): the command line. Four commands; the parameter of
// the app's `start` rule is a flag of each (`--level <n>`), `draw` and `shot` stop at `--at <tick>`, `shot`
// writes its picture to `--out <file.png>`.
#pragma once

#include <optional>
#include <string>
#include <string_view>
#include <variant>
#include <vector>

#include "rt/meta.hpp"

namespace lib::game {

enum class Command { Play, Run, Draw, Shot };

struct Help {
  bool to_stdout = false;
};
struct Usage {
  std::string message;
};
struct Invocation {
  Command command = Command::Run;
  // The arguments of `start`, in order.
  std::vector<rt::Data> start;
  // The last tick to run; absent: the end of the script.
  std::optional<rt::Int> at;
  std::optional<std::string> out;
};
using Parsed = std::variant<Help, Usage, Invocation>;

// `args` excludes the program's name. The first problem from the left is the one reported.
Parsed parse(const rt::meta::App& app, const std::vector<std::string_view>& args);

std::string help_text(const rt::meta::App& app);

// `@bin(name)`, or the app's name.
std::string bin_of(const rt::meta::App& app);

}  // namespace lib::game
