// The `cli` library's runtime for C++ (libs/cli/real/cpp): commands from the manifest. Each rule with
// trigger `cli("name")` is a command; its parameters become arguments:
//   a parameter without a default: a positional argument `<name>`
//   `Bool` with a default: a switch `--name`
//   `Option[T]` or any other type with a default: `--name <value>`, at most once
//   `Set[T]` with a default: `--name <value>`, repeatable; repeats collapse
//   a sum without data: its variant names in lower case
// A value is parsed by its shape, then checked against its declared type: outside it is a usage error.
#pragma once

#include <optional>
#include <string>
#include <string_view>
#include <variant>
#include <vector>

#include "rt/meta.hpp"

namespace lib::cli {

enum class Shape { Positional, Switch, Once, Repeat };

struct Param {
  std::string name;
  Shape shape = Shape::Positional;
  // The type one argument is parsed as: the parameter's, an Option's payload, a Set's element. Null for a switch.
  rt::meta::TypeRef type = nullptr;
  // An Option flag: a parsed value is Some(value).
  bool some = false;
  // This parameter's part of the command's `--help` line.
  std::string usage;
};

struct Command {
  std::string name;
  const rt::meta::Callable* rule = nullptr;
  std::vector<Param> params;
};

// Every rule of the app with a `cli` trigger, in the app's order.
std::vector<Command> commands(const rt::meta::App& app);

std::string help_text(const std::string& bin, const std::vector<Command>& commands);

struct Help {
  bool to_stdout = false;
};
struct Usage {
  std::string message;
};
// The rule's arguments in parameter order; a parameter not given holds its default.
struct Call {
  const Command* command = nullptr;
  std::vector<rt::Data> values;
};
using Parsed = std::variant<Help, Usage, Call>;

// The parsing order of the cli vocabulary: the first problem from the left is the one reported.
Parsed parse(const std::vector<Command>& commands, const std::vector<std::string_view>& args);

// One argument as a value of a declared type, or nothing when it is not one.
std::optional<rt::Data> parse_value(const rt::meta::Type& type, std::string_view text);

// Whether the bytes are well-formed UTF-8 (no surrogate, no overlong form, nothing past U+10FFFF).
bool is_utf8(std::string_view bytes);

}  // namespace lib::cli
