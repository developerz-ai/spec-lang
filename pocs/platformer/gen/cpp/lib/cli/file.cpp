// The `cli` library's runtime for C++ (libs/cli/real/cpp): the store's file and its lock.
#include "lib/cli/file.hpp"

#include <fcntl.h>
#include <signal.h>
#include <sys/file.h>
#include <sys/stat.h>
#include <unistd.h>

#include <cerrno>
#include <thread>
#include <utility>

namespace lib::cli {

UniqueFd::UniqueFd(UniqueFd&& other) noexcept : fd_(std::exchange(other.fd_, -1)) {}

UniqueFd& UniqueFd::operator=(UniqueFd&& other) noexcept {
  if (this != &other) {
    if (fd_ >= 0) ::close(fd_);
    fd_ = std::exchange(other.fd_, -1);
  }
  return *this;
}

UniqueFd::~UniqueFd() {
  // An error here has no reader; close() is the checked path.
  if (fd_ >= 0) ::close(fd_);
}

bool UniqueFd::close() noexcept {
  const int fd = std::exchange(fd_, -1);
  return fd < 0 || ::close(fd) == 0;
}

bool write_all(int fd, std::string_view bytes) {
  while (!bytes.empty()) {
    const ssize_t n = ::write(fd, bytes.data(), bytes.size());
    if (n < 0) {
      if (errno == EINTR) {
        continue;
      }
      return false;
    }
    bytes.remove_prefix(static_cast<std::size_t>(n));
  }
  return true;
}

namespace {

std::string directory_of(const std::string& path) {
  const auto slash = path.rfind('/');
  if (slash == std::string::npos) {
    return ".";
  }
  return slash == 0 ? "/" : path.substr(0, slash);
}

// Removes the temporary file unless the rename consumed it.
class TmpFile {
 public:
  explicit TmpFile(std::string path) : path_(std::move(path)) {}
  TmpFile(const TmpFile&) = delete;
  TmpFile& operator=(const TmpFile&) = delete;
  ~TmpFile() {
    if (!renamed_) {
      ::unlink(path_.c_str());
    }
  }
  [[nodiscard]] const std::string& path() const noexcept {
    return path_;
  }
  void mark_renamed() noexcept {
    renamed_ = true;
  }

 private:
  std::string path_;
  bool renamed_ = false;
};

// Holds back the signals that end a process by default (SIGINT, SIGTERM, SIGHUP, SIGQUIT) while it lives:
// one that arrives meanwhile is delivered when it ends, after the temporary file is renamed or removed.
// So a signal never leaves a temporary file behind or a store half replaced. SIGKILL cannot be held back;
// it can leave the temporary file, never a torn store.
class SignalsHeld {
 public:
  SignalsHeld() noexcept {
    sigset_t held;
    sigemptyset(&held);
    sigaddset(&held, SIGINT);
    sigaddset(&held, SIGTERM);
    sigaddset(&held, SIGHUP);
    sigaddset(&held, SIGQUIT);
    active_ = ::sigprocmask(SIG_BLOCK, &held, &before_) == 0;
  }
  SignalsHeld(const SignalsHeld&) = delete;
  SignalsHeld& operator=(const SignalsHeld&) = delete;
  ~SignalsHeld() {
    if (active_) {
      ::sigprocmask(SIG_SETMASK, &before_, nullptr);
    }
  }

 private:
  sigset_t before_{};
  bool active_ = false;
};

}  // namespace

std::expected<std::optional<std::string>, StoreError> read_file(const std::string& path) {
  UniqueFd fd(::open(path.c_str(), O_RDONLY | O_CLOEXEC));
  if (!fd) {
    if (errno == ENOENT) {
      return std::optional<std::string>{};
    }
    return std::unexpected(StoreError::Unreadable);
  }
  struct stat info{};
  if (::fstat(fd.get(), &info) != 0 || !S_ISREG(info.st_mode)) {
    return std::unexpected(StoreError::Unreadable);
  }
  std::string bytes;
  bytes.resize(static_cast<std::size_t>(info.st_size) + 1);  // +1 sees growth without a 2nd stat
  std::size_t used = 0;
  for (;;) {
    if (used == bytes.size()) {
      bytes.resize(bytes.size() * 2);
    }
    const ssize_t n = ::read(fd.get(), bytes.data() + used, bytes.size() - used);
    if (n < 0) {
      if (errno == EINTR) {
        continue;
      }
      return std::unexpected(StoreError::Unreadable);
    }
    if (n == 0) {
      break;
    }
    used += static_cast<std::size_t>(n);
  }
  bytes.resize(used);
  return std::optional<std::string>(std::move(bytes));
}

std::expected<void, StoreError> replace_atomically(const std::string& path, std::string_view bytes) {
  // Declared first, so it ends last: after the temporary file is renamed or removed on every path.
  const SignalsHeld held;
  TmpFile tmp(path + "." + std::to_string(::getpid()) + ".tmp");
  UniqueFd fd(::open(tmp.path().c_str(), O_WRONLY | O_CREAT | O_TRUNC | O_CLOEXEC, 0666));
  if (!fd || !write_all(fd.get(), bytes) || ::fsync(fd.get()) != 0 || !fd.close()) {
    return std::unexpected(StoreError::Unwritable);
  }
  if (::rename(tmp.path().c_str(), path.c_str()) != 0) {
    return std::unexpected(StoreError::Unwritable);
  }
  tmp.mark_renamed();
  // Make the rename itself durable. EINVAL: the file system cannot sync a directory.
  UniqueFd dir(::open(directory_of(path).c_str(), O_RDONLY | O_DIRECTORY | O_CLOEXEC));
  if (!dir || (::fsync(dir.get()) != 0 && errno != EINVAL)) {
    return std::unexpected(StoreError::Unwritable);
  }
  return {};
}

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

}  // namespace lib::cli
