// Projected by hand from the Lock rule of docs/contract.md section 3, for @store (src/apps/cpp.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "todo/store/lock.hpp"

#include <fcntl.h>
#include <sys/file.h>

#include <cerrno>
#include <thread>

namespace todo::store {

std::expected<Lock, StoreError> acquire(const std::string& lock_path, LockMode mode, LockTiming timing) {
  const int access = mode == LockMode::Exclusive ? O_RDWR : O_RDONLY;
  UniqueFd fd(::open(lock_path.c_str(), access | O_CREAT | O_CLOEXEC, 0666));
  if (!fd) {
    return std::unexpected(StoreError::Unwritable);
  }
  const int operation = (mode == LockMode::Exclusive ? LOCK_EX : LOCK_SH) | LOCK_NB;
  const auto deadline = std::chrono::steady_clock::now() + timing.timeout;
  for (;;) {
    if (::flock(fd.get(), operation) == 0) {
      return Lock(std::move(fd));
    }
    if (errno == EINTR) {
      continue;
    }
    if (errno != EWOULDBLOCK) {
      return std::unexpected(StoreError::Unwritable);
    }
    if (std::chrono::steady_clock::now() >= deadline) {
      return std::unexpected(StoreError::Contended);
    }
    std::this_thread::sleep_for(timing.interval);
  }
}

}  // namespace todo::store
