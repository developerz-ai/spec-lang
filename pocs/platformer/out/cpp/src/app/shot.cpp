// Projected by hand from the shot command the game vocabulary derives from tick and frame: rule step (src/hop/world.slg), rule present (src/hop/screen.slg; docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.
#include "game/app/fnv.hpp"
#include "game/app/program.hpp"
#include "game/app/replay.hpp"
#include "game/cli/exit_code.hpp"
#include "game/script/reader.hpp"
#include "hop/world/rules.hpp"

namespace game::app {

Outcome shot(hop::levels::LevelNo level, std::optional<std::uint32_t> at,
             const std::optional<std::string>& out, std::string_view input, const std::string& assets_dir) {
  const auto script = script::read(input);
  if (!script) {
    return {cli::exit_usage, "", script.error() + "\n"};
  }
  hop::world::World world;
  hop::world::Lines silent = hop::world::Lines::dropped();
  hop::world::start(world, level, silent);
  replay(world, *script, at, silent);

  auto session = hop::screen::Session::open(hop::screen::Purpose::Shot, shot_window, assets_dir);
  if (!session) {
    return {cli::exit_screen, "", "error: " + session.error() + "\n"};
  }
  const hop::screen::Capture frame = session->capture(world, shot_window.width, shot_window.height, out);
  // Open choices (NOTES.md): the contract names no error for a frame that cannot be read back or
  // a PNG that cannot be written; both end like the other screen failures, with nothing on stdout.
  if (frame.rgba.empty()) {
    return {cli::exit_screen, "", "error: window: cannot read the frame\n"};
  }
  if (!frame.exported) {
    return {cli::exit_screen, "", "error: " + *out + ": cannot write\n"};
  }
  return {cli::exit_ok, "shot " + hex16(fnv1a64(frame.rgba)) + "\n", ""};
}

}  // namespace game::app
