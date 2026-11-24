// Projected by hand from the argument rules of the game vocabulary (docs/contract.md section 2, "Parsing, in this order") for app SkyhopCpp (src/apps/cpp.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <span>
#include <string_view>

#include "game/cli/command.hpp"

namespace game::cli {

// args: the command line without the program name.
Parsed parse(std::span<const std::string_view> args);

}  // namespace game::cli
