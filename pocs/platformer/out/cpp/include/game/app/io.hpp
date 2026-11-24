// Projected by hand from the streams of the game vocabulary (docs/contract.md section 2): standard input read whole, out.line to standard output. Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <string>
#include <string_view>

namespace game::app {

// All of standard input, as bytes.
std::string read_stdin();

// Writes and flushes; false when the stream refused.
bool write_stdout(std::string_view text);
void write_stderr(std::string_view text);

}  // namespace game::app
