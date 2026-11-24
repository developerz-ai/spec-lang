// Projected by hand from @store(file(".todo.json")) (src/apps/cpp.slg): load before the rule, atomic replace after it (docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.
#include "todo/store/file.hpp"

#include <fcntl.h>
#include <sys/stat.h>
#include <unistd.h>

#include <cerrno>
#include <utility>

#include "todo/store/fd.hpp"

namespace todo::store {
namespace {

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
  [[nodiscard]] const std::string& path() const noexcept { return path_; }
  void mark_renamed() noexcept { renamed_ = true; }

 private:
  std::string path_;
  bool renamed_ = false;
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

}  // namespace todo::store
