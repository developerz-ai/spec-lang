// Projected by hand from app TodoCpp, @bin("todo") (src/apps/cpp.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include <unistd.h>

#include <cerrno>
#include <cstdlib>
#include <string>
#include <string_view>
#include <vector>

#include "todo/app/program.hpp"

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

}  // namespace

int main(int argc, char** argv) {
  const std::vector<std::string_view> args(argv + 1, argv + argc);
  const char* configured = std::getenv("TODO_STORE");
  const std::string store_path = (configured != nullptr && *configured != '\0') ? configured : ".todo.json";
  const todo::app::RunResult r = todo::app::run(args, store_path);
  if (!write_all(STDERR_FILENO, r.err)) {
    return r.code == 0 ? 1 : r.code;
  }
  if (!write_all(STDOUT_FILENO, r.out)) {
    write_all(STDERR_FILENO, "error: stdout: unwritable\n");  // nothing left to report a failure to
    return 1;
  }
  return r.code;
}
