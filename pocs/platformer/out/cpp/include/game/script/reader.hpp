// Projected by hand from "The input script" (docs/contract.md section 2) for rule step(pad: Pad) (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <expected>
#include <string>
#include <string_view>

#include "game/script/script.hpp"

namespace game::script {

// Reads and checks the whole of standard input before the first tick. The error is the line for
// standard error without its newline: "error: input line N: <why>" or
// "error: input: more than 1000000 ticks".
std::expected<Script, std::string> read(std::string_view input);

}  // namespace game::script
