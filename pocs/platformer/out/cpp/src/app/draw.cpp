// Projected by hand from the draw command the game vocabulary derives from tick and dump: rule step (src/hop/world.slg), fn dump (src/hop/render.slg; docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.
#include "game/app/program.hpp"
#include "game/app/replay.hpp"
#include "game/cli/exit_code.hpp"
#include "game/script/reader.hpp"
#include "hop/render/text.hpp"
#include "hop/world/rules.hpp"

namespace game::app {

Outcome draw(hop::levels::LevelNo level, std::optional<std::uint32_t> at, std::string_view input) {
  const auto script = script::read(input);
  if (!script) {
    return {cli::exit_usage, "", script.error() + "\n"};
  }
  hop::world::World world;
  hop::world::Lines silent = hop::world::Lines::dropped();
  hop::world::start(world, level, silent);
  replay(world, *script, at, silent);
  std::string out;
  for (const std::string& line : hop::render::dump(world)) {
    out += line;
    out += '\n';
  }
  return {cli::exit_ok, std::move(out), ""};
}

}  // namespace game::app
