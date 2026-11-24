// The `cli` library's runtime for C++ (libs/cli/real/cpp): one run of an app. The arguments choose a rule;
// the store is locked from before the read until after the write, read strictly (its types, then the
// program's laws about a world alone) and replaced atomically when the rule changed it; the rule's lines
// are printed only after that.
#include <unistd.h>

#include <algorithm>
#include <cctype>
#include <cstdlib>

#include "lib/cli/args.hpp"
#include "lib/cli/codec.hpp"
#include "lib/cli/file.hpp"
#include "lib/cli/json.hpp"
#include "lib/cli/main.hpp"
#include "lib/cli/out.hpp"
#include "program/machine.hpp"

namespace lib::cli {

namespace {

Output failure(std::string_view message, int code) {
  return {{}, "error: " + std::string{message} + "\n", code};
}

Output store_failure(StoreError error, const std::string& path) {
  switch (error) {
    case StoreError::Contended:
      return failure("Contended", kRefused);
    case StoreError::Unreadable:
      return failure("store " + path + ": unreadable", kStore);
    case StoreError::Unwritable:
      return failure("store " + path + ": unwritable", kStore);
    case StoreError::Full:
      return failure("store " + path + ": full", kStore);
  }
  return failure("store " + path + ": unreadable", kStore);
}

// `@store(file(path))`: the path, or nothing when the app keeps no store.
std::optional<std::string> store_file(const rt::meta::App& app) {
  const rt::meta::Note* note = app.note("cli", "store");
  const rt::meta::NoteValue* at = note == nullptr ? nullptr : note->arg("at");
  const rt::meta::NoteValue* path = at == nullptr || at->text != "file" ? nullptr : at->arg("path");
  return path == nullptr ? std::nullopt : std::optional<std::string>{path->text};
}

std::string bin_of(const rt::meta::App& app) {
  const rt::meta::Note* note = app.note("cli", "bin");
  const rt::meta::NoteValue* name = note == nullptr ? nullptr : note->arg("name");
  return name == nullptr ? app.name : name->text;
}

// The stored bytes into the machine's world. False: not a world of the program.
bool load(const rt::meta::App& app, program::Machine& machine, const std::string& bytes) {
  if (!is_utf8(bytes)) return false;
  const auto json = read_json(bytes);
  if (!json) return false;
  try {
    app.set_world(machine, decode_world(app, *json));
    return app.lawful(machine);
  } catch (const Unfit&) {
  } catch (const rt::Failure&) {
  } catch (const rt::Stop&) {
  }
  return false;
}

// The rule on the machine: nothing when it ran, else what to report.
std::optional<Output> invoke(const Call& call, program::Machine& machine) {
  try {
    call.command->rule->call(machine, call.values);
  } catch (const rt::Failure& refused) {
    return failure(refused.name, kRefused);
  } catch (const rt::Stop& stopped) {
    return failure(stopped.code + ": " + stopped.message, kRefused);
  }
  return std::nullopt;
}

Output with_store(const rt::meta::App& app, const Call& call, program::Machine& machine, const std::string& path) {
  const bool writes = call.command->rule->writes;
  const auto lock = acquire(path + ".lock", writes ? LockMode::Exclusive : LockMode::Shared);
  if (!lock) return store_failure(lock.error(), path);
  const auto bytes = read_file(path);
  if (!bytes) return store_failure(bytes.error(), path);
  // A missing store is the initial world.
  if (bytes->has_value() && !load(app, machine, **bytes)) return store_failure(StoreError::Unreadable, path);
  if (auto refused = invoke(call, machine)) return *refused;
  // A rule that only reads holds the shared lock and never writes the store.
  if (writes) {
    std::string encoded;
    try {
      encoded = encode_world(app, app.world(machine));
    } catch (const Full&) {
      return store_failure(StoreError::Full, path);
    }
    // Byte-identical: the commit is already on disk, so the replace is skipped.
    if (!bytes->has_value() || **bytes != encoded) {
      if (const auto replaced = replace_atomically(path, encoded); !replaced) {
        return store_failure(replaced.error(), path);
      }
    }
  }
  return {take_lines(machine), {}, 0};
}

}  // namespace

Output run(const rt::meta::App& app, const std::vector<std::string_view>& args, const std::string& store_override) {
  const std::string bin = bin_of(app);
  const auto all = commands(app);
  const Parsed parsed = parse(all, args);
  if (const auto* help = std::get_if<Help>(&parsed)) {
    return help->to_stdout ? Output{help_text(bin, all), {}, 0} : Output{{}, help_text(bin, all), kUsage};
  }
  if (const auto* usage = std::get_if<Usage>(&parsed)) return failure(usage->message, kUsage);
  const Call& call = std::get<Call>(parsed);

  program::Machine machine;
  machine.out = std::make_unique<BufferedOut>();
  const auto file = store_file(app);
  if (!file) {
    if (auto refused = invoke(call, machine)) return *refused;
    return {take_lines(machine), {}, 0};
  }
  return with_store(app, call, machine, store_override.empty() ? *file : store_override);
}

int main(const rt::meta::App& app, int argc, char** argv) {
  const std::vector<std::string_view> args(argv + std::min(argc, 1), argv + argc);
  // `<BIN>_STORE` replaces the store's path; empty counts as unset.
  std::string variable = bin_of(app) + "_STORE";
  std::ranges::transform(variable, variable.begin(),
                         [](unsigned char c) { return static_cast<char>(std::toupper(c)); });
  const char* configured = std::getenv(variable.c_str());
  const Output result = run(app, args, configured == nullptr ? std::string{} : std::string{configured});
  if (!write_all(STDERR_FILENO, result.err)) return result.code == 0 ? 1 : result.code;
  if (!write_all(STDOUT_FILENO, result.out)) {
    write_all(STDERR_FILENO, "error: stdout: unwritable\n");  // nothing left to report a failure to
    return 1;
  }
  return result.code;
}

}  // namespace lib::cli
