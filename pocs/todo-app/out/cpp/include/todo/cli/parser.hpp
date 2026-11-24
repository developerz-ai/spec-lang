// Projected by hand from the cli vocabulary's derivations (docs/cli-vocabulary.md) and docs/contract.md section 2. Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <optional>
#include <span>
#include <string>
#include <string_view>
#include <variant>

#include "todo/cli/command.hpp"
#include "todo/cli/params.hpp"

namespace todo::cli {

struct HelpRequest {
  bool to_stdout;  // --help or -h: stdout, exit 0. No arguments: stderr, exit 2.
};

struct UsageError {
  std::string message;  // without "error: " and without the newline
};

using Parsed = std::variant<Command, HelpRequest, UsageError>;

// TaskNo: ASCII digits, at least one, value at least 1, leading zeros allowed. Values past 64 bits
// saturate: every task number in a store is at most 2^53, so a saturated one is never in it.
[[nodiscard]] std::optional<model::TaskNo> parse_task_no(std::string_view text) noexcept;

[[nodiscard]] std::optional<model::Priority> parse_priority(std::string_view text) noexcept;

// args excludes the program name.
[[nodiscard]] Parsed parse(std::span<const std::string_view> args);

}  // namespace todo::cli
