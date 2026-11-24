// Projected by hand from app TodoCpp (src/apps/cpp.slg) with the cli vocabulary (docs/cli-vocabulary.md). Stand-in for spec-lang build output; change the spec, not this file.
#include "todo/app/program.hpp"

#include <optional>
#include <utility>
#include <variant>

#include "todo/board/rules.hpp"
#include "todo/cli/dispatch.hpp"
#include "todo/cli/exit_code.hpp"
#include "todo/cli/help.hpp"
#include "todo/cli/parser.hpp"
#include "todo/store/codec.hpp"
#include "todo/store/file.hpp"
#include "todo/store/lock.hpp"

namespace todo::app {
namespace {

using cli::ExitCode;

RunResult fail(ExitCode code, std::string_view message) {
  std::string err = "error: ";
  err += message;
  err += '\n';
  return {static_cast<int>(code), {}, std::move(err)};
}

RunResult store_failure(store::StoreError e, const std::string& path) {
  switch (e) {
    case store::StoreError::Contended:
      return fail(ExitCode::Refused, "Contended");
    case store::StoreError::Unreadable:
      return fail(ExitCode::Store, "store " + path + ": unreadable");
    case store::StoreError::Unwritable:
      return fail(ExitCode::Store, "store " + path + ": unwritable");
    case store::StoreError::Full:
      return fail(ExitCode::Store, "store " + path + ": full");
  }
  return fail(ExitCode::Store, "store " + path + ": unreadable");
}

RunResult lines_out(const std::vector<std::string>& lines) {
  RunResult r;
  for (const std::string& l : lines) {
    r.out += l;
    r.out += '\n';
  }
  return r;
}

// Runs a parsed command under the lock. The lock is released when this returns.
RunResult locked(const cli::Command& command, const std::string& path) {
  const bool writes = cli::access_of(command) == cli::Access::Write;
  const auto lock =
      store::acquire(path + ".lock", writes ? store::LockMode::Exclusive : store::LockMode::Shared);
  if (!lock) {
    return store_failure(lock.error(), path);
  }
  const auto bytes = store::read_file(path);
  if (!bytes) {
    return store_failure(bytes.error(), path);
  }
  // stats reads only status: count while validating, build nothing.
  if (std::holds_alternative<cli::Stats>(command)) {
    const auto counts = bytes->has_value() ? store::count_statuses(**bytes) : store::StatusCounts{};
    if (!counts) {
      return store_failure(store::StoreError::Unreadable, path);
    }
    return lines_out({board::stats_line(counts->open, counts->done)});
  }
  std::optional<board::World> world = bytes->has_value() ? store::decode(**bytes) : board::World{};
  if (!world) {
    return store_failure(store::StoreError::Unreadable, path);
  }
  cli::Applied applied = cli::apply(command, *world);
  if (applied.refused) {
    return fail(ExitCode::Refused, board::name(*applied.refused));
  }
  if (applied.written) {
    const auto encoded = store::encode(*applied.written);
    if (!encoded) {
      return store_failure(encoded.error(), path);
    }
    // A byte-identical store is not rewritten: the world on disk is already the new one.
    if (!bytes->has_value() || **bytes != *encoded) {
      if (const auto replaced = store::replace_atomically(path, *encoded); !replaced) {
        return store_failure(replaced.error(), path);
      }
    }
  }
  return lines_out(applied.lines);
}

}  // namespace

RunResult run(std::span<const std::string_view> args, const std::string& store_path) {
  cli::Parsed parsed = cli::parse(args);
  if (const auto* help = std::get_if<cli::HelpRequest>(&parsed)) {
    if (help->to_stdout) {
      return {static_cast<int>(ExitCode::Ok), std::string(cli::help_text), {}};
    }
    return {static_cast<int>(ExitCode::Usage), {}, std::string(cli::help_text)};
  }
  if (const auto* usage = std::get_if<cli::UsageError>(&parsed)) {
    return fail(ExitCode::Usage, usage->message);
  }
  return locked(std::get<cli::Command>(parsed), store_path);
}

}  // namespace todo::app
