// The `cli` library's runtime for C++ (libs/cli/real/cpp): the store's file. Reading it, replacing it
// atomically, and the lock held around one rule: flock(2) on `<path>.lock`, tried every 10 ms for 5 s.
#pragma once

#include <chrono>
#include <expected>
#include <optional>
#include <string>
#include <string_view>

namespace lib::cli {

// What can go wrong between a rule and the disk; none of these is a refusal of the program.
enum class StoreError { Unreadable, Unwritable, Full, Contended };

// Owns one file descriptor; closes it once.
class UniqueFd {
 public:
  UniqueFd() = default;
  explicit UniqueFd(int fd) noexcept : fd_(fd) {}
  UniqueFd(const UniqueFd&) = delete;
  UniqueFd& operator=(const UniqueFd&) = delete;
  UniqueFd(UniqueFd&& other) noexcept;
  UniqueFd& operator=(UniqueFd&& other) noexcept;
  ~UniqueFd();

  [[nodiscard]] int get() const noexcept {
    return fd_;
  }
  [[nodiscard]] explicit operator bool() const noexcept {
    return fd_ >= 0;
  }
  // Closes now and reports whether close succeeded (it can report a deferred write error).
  [[nodiscard]] bool close() noexcept;

 private:
  int fd_ = -1;
};

// The file's bytes; nullopt when it does not exist (the initial world). Other failures: Unreadable.
[[nodiscard]] std::expected<std::optional<std::string>, StoreError> read_file(const std::string& path);

// Writes <path>.<pid>.tmp beside path, fsyncs it, renames it over path, fsyncs the directory.
// Any failure is Unwritable and leaves no temporary file behind.
[[nodiscard]] std::expected<void, StoreError> replace_atomically(const std::string& path, std::string_view bytes);

// Shared for a rule that only reads; exclusive for one that writes.
enum class LockMode { Shared, Exclusive };

struct LockTiming {
  std::chrono::milliseconds timeout{5000};
  std::chrono::milliseconds interval{10};
};

// flock(2) on a lock file, held until destroyed. The file is created if missing, never deleted.
class Lock {
 public:
  explicit Lock(UniqueFd fd) noexcept : fd_(std::move(fd)) {}

 private:
  UniqueFd fd_;  // closing the descriptor releases the lock
};

// Tries without blocking every interval until timeout: Contended. Cannot open: Unwritable.
[[nodiscard]] std::expected<Lock, StoreError> acquire(const std::string& lock_path, LockMode mode,
                                                      LockTiming timing = {});

// Every byte to a descriptor; false when the descriptor refuses.
bool write_all(int fd, std::string_view bytes);

}  // namespace lib::cli
