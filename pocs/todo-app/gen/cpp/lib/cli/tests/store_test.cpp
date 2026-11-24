// The `cli` library's runtime for C++ (libs/cli/real/cpp): its own tests, over the manifest of the app it was
// built into. The store's format round trips, the reader is strict, the file is replaced whole, and a lock
// that is held is reported, never waited out silently.
#include <unistd.h>

#include <cstdlib>
#include <filesystem>
#include <iostream>
#include <string>

#include "lib/cli/args.hpp"
#include "lib/cli/codec.hpp"
#include "lib/cli/file.hpp"
#include "lib/cli/json.hpp"
#include "lib/cli/main.hpp"
#include "program/machine.hpp"
#include "program/manifest.hpp"

namespace {

int failures = 0;

void expect(bool holds, const char* what) {
  if (!holds) {
    ++failures;
    std::cerr << "FAIL " << what << "\n";
  }
}

// A directory of its own, removed when the test ends.
class TempDir {
 public:
  TempDir() {
    std::string pattern = (std::filesystem::temp_directory_path() / "lib-cli-XXXXXX").string();
    if (::mkdtemp(pattern.data()) == nullptr) std::abort();
    path_ = pattern;
  }
  TempDir(const TempDir&) = delete;
  TempDir& operator=(const TempDir&) = delete;
  ~TempDir() {
    std::error_code ignored;
    std::filesystem::remove_all(path_, ignored);
  }
  std::string file(const char* name) const {
    return (path_ / name).string();
  }

 private:
  std::filesystem::path path_;
};

void the_initial_world_round_trips() {
  const auto& app = program::app();
  const program::Machine machine;
  const std::string bytes = lib::cli::encode_world(app, app.world(machine));
  expect(bytes.ends_with("}\n") && bytes.starts_with("{\"version\":1"), "the store is one line, version first");
  const auto json = lib::cli::read_json(bytes);
  expect(json.has_value(), "what was written is JSON");
  if (!json) return;
  expect(lib::cli::encode_world(app, lib::cli::decode_world(app, *json)) == bytes,
         "a read then a write gives the same bytes");
}

void the_reader_is_strict() {
  using lib::cli::read_json;
  expect(!read_json(""), "an empty file is not JSON");
  expect(!read_json("{\"a\":1,\"a\":2}"), "a key twice is refused");
  expect(!read_json("\"\\ud800\""), "a lone surrogate is refused");
  expect(!read_json("[1,]"), "a trailing comma is refused");
  expect(!read_json("01"), "a leading zero is refused");
  expect(!read_json("{} x"), "trailing characters are refused");
  const auto exact = read_json("9007199254740993");
  expect(exact && exact->integer == rt::Int{9007199254740993LL}, "an integer above 2^53 is kept exact");
  const auto fraction = read_json("1.0");
  expect(fraction && !fraction->integer, "1.0 is not an integer");
  const auto pair = read_json("\"\\ud83d\\ude00\"");
  expect(pair && pair->string == "\xF0\x9F\x98\x80", "a surrogate pair is one code point");
  expect(!lib::cli::is_utf8("\xC0\x80") && !lib::cli::is_utf8("\xED\xA0\x80") && lib::cli::is_utf8("\xE4\xB8\xAD"),
         "UTF-8 is checked strictly");
}

void a_store_of_another_shape_is_unreadable() {
  const auto& app = program::app();
  for (const char* text : {"{}", "[]", "{\"version\":2}", "{\"version\":1,\"no such state\":0}"}) {
    const auto json = lib::cli::read_json(text);
    bool refused = false;
    try {
      if (json) lib::cli::decode_world(app, *json);
    } catch (const lib::cli::Unfit&) {
      refused = true;
    }
    expect(refused || app.states.empty(), "a document of another shape is not a world");
  }
}

void the_file_is_replaced_whole() {
  const TempDir dir;
  const std::string path = dir.file("store.json");
  const auto missing = lib::cli::read_file(path);
  expect(missing && !missing->has_value(), "a missing store reads as absent");
  expect(lib::cli::replace_atomically(path, "one\n").has_value(), "the first write succeeds");
  expect(lib::cli::replace_atomically(path, "two\n").has_value(), "a second write replaces it");
  const auto read = lib::cli::read_file(path);
  expect(read && read->has_value() && **read == "two\n", "the file holds the last bytes written");
  std::size_t files = 0;
  for ([[maybe_unused]] const auto& entry :
       std::filesystem::directory_iterator(std::filesystem::path(path).parent_path())) {
    ++files;
  }
  expect(files == 1, "no temporary file is left behind");
  expect(!lib::cli::replace_atomically(dir.file("no/such/dir/store.json"), "x").has_value(),
         "a directory that does not exist is unwritable");
  expect(!lib::cli::read_file(dir.file("")).has_value(), "a directory is not a readable store");
}

void a_held_lock_is_contended() {
  const TempDir dir;
  const std::string path = dir.file("store.json.lock");
  const lib::cli::LockTiming brief{std::chrono::milliseconds{30}, std::chrono::milliseconds{5}};
  {
    const auto held = lib::cli::acquire(path, lib::cli::LockMode::Exclusive, brief);
    expect(held.has_value(), "a free lock is had");
    const auto second = lib::cli::acquire(path, lib::cli::LockMode::Shared, brief);
    expect(!second && second.error() == lib::cli::StoreError::Contended, "a held lock is Contended after the wait");
  }
  const auto shared = lib::cli::acquire(path, lib::cli::LockMode::Shared, brief);
  const auto also = lib::cli::acquire(path, lib::cli::LockMode::Shared, brief);
  expect(shared.has_value() && also.has_value(), "two readers share the lock once it is released");
}

void help_and_usage_need_no_store() {
  const auto& app = program::app();
  const auto none = lib::cli::run(app, {}, "/nonexistent/dir/store.json");
  expect(none.code == lib::cli::kUsage && none.out.empty() && none.err.starts_with("usage: "),
         "no arguments: help on standard error, exit 2");
  const auto help = lib::cli::run(app, {"--help"}, "/nonexistent/dir/store.json");
  expect(help.code == 0 && help.err.empty() && help.out == none.err,
         "--help: the same text on standard output, exit 0");
  const auto unknown = lib::cli::run(app, {"no-such-command"}, "/nonexistent/dir/store.json");
  expect(unknown.code == lib::cli::kUsage && unknown.err == "error: unknown command \"no-such-command\"\n",
         "an unknown command is a usage error");
}

}  // namespace

int main() {
  the_initial_world_round_trips();
  the_reader_is_strict();
  a_store_of_another_shape_is_unreadable();
  the_file_is_replaced_whole();
  a_held_lock_is_contended();
  help_and_usage_need_no_store();
  if (failures == 0) std::cout << "lib/cli: every test passed\n";
  return failures == 0 ? 0 : 1;
}
