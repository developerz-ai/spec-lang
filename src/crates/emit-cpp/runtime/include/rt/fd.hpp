// An owned file descriptor.
#pragma once

#include <unistd.h>

#include <utility>

namespace rt {

// Owns one file descriptor; closes it once.
class UniqueFd {
 public:
  UniqueFd() = default;
  explicit UniqueFd(int fd) noexcept : fd_(fd) {}
  UniqueFd(const UniqueFd&) = delete;
  UniqueFd& operator=(const UniqueFd&) = delete;
  UniqueFd(UniqueFd&& other) noexcept : fd_(std::exchange(other.fd_, -1)) {}
  UniqueFd& operator=(UniqueFd&& other) noexcept {
    if (this != &other) {
      reset();
      fd_ = std::exchange(other.fd_, -1);
    }
    return *this;
  }
  ~UniqueFd() { reset(); }

  [[nodiscard]] int get() const noexcept { return fd_; }
  [[nodiscard]] explicit operator bool() const noexcept { return fd_ >= 0; }

  // Closes now and reports whether close succeeded (it can report a deferred write error).
  [[nodiscard]] bool close() noexcept {
    const int fd = std::exchange(fd_, -1);
    return fd < 0 || ::close(fd) == 0;
  }

 private:
  void reset() noexcept {
    if (fd_ >= 0) {
      ::close(fd_);  // an error here has no reader; close() above is the checked path
      fd_ = -1;
    }
  }
  int fd_ = -1;
};

}  // namespace rt
