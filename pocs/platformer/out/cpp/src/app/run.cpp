// Projected by hand from the run command the game vocabulary derives from tick and report: rule step, fn summary (src/hop/world.slg; docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.
#include "game/app/program.hpp"
#include "game/app/replay.hpp"
#include "game/cli/exit_code.hpp"
#include "game/script/reader.hpp"
#include "hop/world/rules.hpp"

namespace game::app {

Outcome run(hop::levels::LevelNo level, std::string_view input) {
  const auto script = script::read(input);
  if (!script) {
    return {cli::exit_usage, "", script.error() + "\n"};
  }
  hop::world::World world;
  hop::world::Lines lines = hop::world::Lines::kept();
  hop::world::start(world, level, lines);
  replay(world, *script, std::nullopt, lines);
  lines.line(hop::world::summary(world));
  return {cli::exit_ok, lines.take(), ""};
}

}  // namespace game::app
