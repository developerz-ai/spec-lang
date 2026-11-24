// Projected by hand from @store(file(".todo.json")) (src/apps/cpp.slg): lock, load and atomic replace of docs/contract.md section 3. Stand-in for spec-lang build output; change the spec, not this file.
#include <sys/stat.h>

#include <chrono>
#include <filesystem>
#include <fstream>
#include <iterator>
#include <string>

#include "harness.hpp"
#include "temp_dir.hpp"
#include "todo/store/file.hpp"
#include "todo/store/lock.hpp"

using todo::store::LockMode;
using todo::store::StoreError;

namespace {

std::string slurp(const std::string& path) {
  std::ifstream in(path, std::ios::binary);
  return {std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>()};
}

constexpr todo::store::LockTiming quick{std::chrono::milliseconds(60), std::chrono::milliseconds(10)};

}  // namespace

TEST(missing_store_is_the_initial_world) {
  const support::TempDir dir;
  const auto r = todo::store::read_file(dir.file(".todo.json"));
  CHECK(r.has_value());
  CHECK(!r->has_value());
}

TEST(a_directory_is_unreadable) {
  const support::TempDir dir;
  std::filesystem::create_directory(dir.file("d"));
  CHECK(todo::store::read_file(dir.file("d")).error() == StoreError::Unreadable);
}

TEST(replace_writes_exact_bytes_and_leaves_no_tmp) {
  const support::TempDir dir;
  const std::string path = dir.file("s.json");
  CHECK(todo::store::replace_atomically(path, "one\n").has_value());
  CHECK(todo::store::replace_atomically(path, "two\n").has_value());
  CHECK_EQ(slurp(path), "two\n");
  CHECK_EQ(**todo::store::read_file(path), "two\n");
  int entries = 0;
  for ([[maybe_unused]] const auto& e : std::filesystem::directory_iterator(dir.path())) {
    ++entries;
  }
  CHECK_EQ(entries, 1);
}

TEST(replace_into_a_missing_directory_is_unwritable) {
  const support::TempDir dir;
  CHECK(todo::store::replace_atomically(dir.file("no/such.json"), "x\n").error() == StoreError::Unwritable);
}

TEST(lock_creates_its_file_and_keeps_it) {
  const support::TempDir dir;
  const std::string lock = dir.file(".todo.json.lock");
  {
    const auto held = todo::store::acquire(lock, LockMode::Shared);
    CHECK(held.has_value());
  }
  CHECK(std::filesystem::exists(lock));
}

TEST(lock_in_a_missing_directory_is_unwritable) {
  const support::TempDir dir;
  CHECK(todo::store::acquire(dir.file("no/x.lock"), LockMode::Shared).error() == StoreError::Unwritable);
}

TEST(exclusive_excludes_everyone_until_released) {
  const support::TempDir dir;
  const std::string lock = dir.file("l");
  {
    const auto writer = todo::store::acquire(lock, LockMode::Exclusive);
    CHECK(writer.has_value());
    CHECK(todo::store::acquire(lock, LockMode::Exclusive, quick).error() == StoreError::Contended);
    CHECK(todo::store::acquire(lock, LockMode::Shared, quick).error() == StoreError::Contended);
  }
  CHECK(todo::store::acquire(lock, LockMode::Exclusive, quick).has_value());
}

TEST(shared_readers_share) {
  const support::TempDir dir;
  const std::string lock = dir.file("l");
  const auto a = todo::store::acquire(lock, LockMode::Shared);
  const auto b = todo::store::acquire(lock, LockMode::Shared, quick);
  CHECK(a.has_value());
  CHECK(b.has_value());
  CHECK(todo::store::acquire(lock, LockMode::Exclusive, quick).error() == StoreError::Contended);
}

TEST(contended_waits_about_the_timeout) {
  const support::TempDir dir;
  const std::string lock = dir.file("l");
  const auto writer = todo::store::acquire(lock, LockMode::Exclusive);
  const auto started = std::chrono::steady_clock::now();
  CHECK(!todo::store::acquire(lock, LockMode::Exclusive, quick).has_value());
  CHECK(std::chrono::steady_clock::now() - started >= quick.timeout);
}
