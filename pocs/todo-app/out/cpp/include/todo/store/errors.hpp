// Projected by hand from @store(file(".todo.json")) (src/apps/cpp.slg) and its errors in docs/contract.md sections 2 and 3. Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <cstdint>

namespace todo::store {

enum class StoreError : std::uint8_t { Unreadable, Unwritable, Full, Contended };

}  // namespace todo::store
