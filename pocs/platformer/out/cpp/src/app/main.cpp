// Projected by hand from app SkyhopCpp, @target(cpp), @bin("skyhop") (src/apps/cpp.slg): the process edge, argv and SKYHOP_ASSETS (docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.
#include <cstdlib>
#include <string>
#include <string_view>
#include <vector>

#include "game/app/io.hpp"
#include "game/app/program.hpp"
#include "game/cli/parser.hpp"

int main(int argc, char** argv) {
  const std::vector<std::string_view> args(argv + (argc > 0 ? 1 : 0), argv + argc);
  // SKYHOP_ASSETS wins; otherwise assets/ relative to the working directory.
  const char* const env = std::getenv("SKYHOP_ASSETS");
  const std::string assets = env != nullptr && *env != '\0' ? env : "assets";
  return game::app::main_program(game::cli::parse(args), &game::app::read_stdin, assets);
}
