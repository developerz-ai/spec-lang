// Projected by hand from @store(file(".todo.json")) (src/apps/cpp.slg): load before the rule, atomic replace after it (docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <expected>
#include <optional>
#include <string>
#include <string_view>

#include "todo/store/errors.hpp"

namespace todo::store {

// The file's bytes; nullopt when it does not exist (the initial world). Other failures: Unreadable.
[[nodiscard]] std::expected<std::optional<std::string>, StoreError> read_file(const std::string& path);

// Writes <path>.<pid>.tmp beside path, fsyncs it, renames it over path, fsyncs the directory.
// Any failure is Unwritable and leaves no temporary file behind.
[[nodiscard]] std::expected<void, StoreError> replace_atomically(const std::string& path,
                                                                 std::string_view bytes);

}  // namespace todo::store
