// Projected by hand from "one lock around it" (docs/cli-vocabulary.md) and the Lock rule of docs/contract.md section 3, for @store (src/apps/cpp.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <chrono>
#include <expected>
#include <string>

#include "todo/store/errors.hpp"
#include "todo/store/fd.hpp"

namespace todo::store {

// Shared for a rule whose @uses only reads; exclusive for one that writes.
enum class LockMode : std::uint8_t { Shared, Exclusive };

struct LockTiming {
  std::chrono::milliseconds timeout{5000};
  std::chrono::milliseconds interval{10};
};

// flock(2) on lock_path, held until destroyed. The file is created if missing, never deleted.
class Lock {
 public:
  explicit Lock(UniqueFd fd) noexcept : fd_(std::move(fd)) {}

 private:
  UniqueFd fd_;  // closing the descriptor releases the lock
};

// Tries without blocking every interval until timeout: Contended. Cannot open: Unwritable.
[[nodiscard]] std::expected<Lock, StoreError> acquire(const std::string& lock_path, LockMode mode,
                                                      LockTiming timing = {});

}  // namespace todo::store
