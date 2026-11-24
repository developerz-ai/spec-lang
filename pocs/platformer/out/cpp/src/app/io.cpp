// Projected by hand from the streams of the game vocabulary (docs/contract.md section 2): standard input read whole, out.line to standard output. Stand-in for spec-lang build output; change the spec, not this file.
#include "game/app/io.hpp"

#include <cstdio>

namespace game::app {

std::string read_stdin() {
  std::string input;
  char chunk[65536];
  for (;;) {
    const std::size_t n = std::fread(chunk, 1, sizeof chunk, stdin);
    input.append(chunk, n);
    if (n < sizeof chunk) {
      // End of input, or a read error: what was read is the script, and the reader judges it.
      return input;
    }
  }
}

bool write_stdout(std::string_view text) {
  return std::fwrite(text.data(), 1, text.size(), stdout) == text.size() && std::fflush(stdout) == 0;
}

void write_stderr(std::string_view text) { std::fwrite(text.data(), 1, text.size(), stderr); }

}  // namespace game::app
