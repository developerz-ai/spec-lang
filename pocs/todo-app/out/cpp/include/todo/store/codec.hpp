// Projected by hand from state tasks, state next_no (src/todo/board.slg) under @store (src/apps/cpp.slg), in the format of docs/contract.md section 3. Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <cstddef>
#include <cstdint>
#include <expected>
#include <optional>
#include <string>
#include <string_view>

#include "todo/board/world.hpp"
#include "todo/store/errors.hpp"

namespace todo::store {

// The largest next_no a store may hold: 2^53.
inline constexpr std::uint64_t max_next_no = std::uint64_t{1} << 53U;

// The whole world, or nullopt when the bytes are not a store (any rule of section 3 broken).
[[nodiscard]] std::optional<board::World> decode(std::string_view bytes);

struct StatusCounts {
  std::size_t open = 0;
  std::size_t done = 0;
};

// Checks every rule decode checks, but keeps only how many tasks are Open and Done.
[[nodiscard]] std::optional<StatusCounts> count_statuses(std::string_view bytes);

// Canonical bytes, newline included. Full when next_no is above 2^53.
[[nodiscard]] std::expected<std::string, StoreError> encode(const board::World& world);

}  // namespace todo::store
