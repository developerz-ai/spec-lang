// Projected by hand from @store(file(".todo.json")) (src/apps/cpp.slg): a temporary directory per store test, no mocks. Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <unistd.h>

#include <cstdlib>
#include <filesystem>
#include <string>

#include "harness.hpp"

namespace support {

class TempDir {
 public:
  TempDir() {
    std::string pattern = (std::filesystem::temp_directory_path() / "todo-cpp-test-XXXXXX").string();
    harness::check(::mkdtemp(pattern.data()) != nullptr, "mkdtemp");
    path_ = pattern;
  }
  TempDir(const TempDir&) = delete;
  TempDir& operator=(const TempDir&) = delete;
  ~TempDir() {
    std::error_code ignored;  // cleanup of a test directory; a leftover is harmless
    std::filesystem::remove_all(path_, ignored);
  }
  [[nodiscard]] std::string file(const std::string& name) const { return (path_ / name).string(); }
  [[nodiscard]] const std::filesystem::path& path() const { return path_; }

 private:
  std::filesystem::path path_;
};

}  // namespace support
