// Projected by hand from the play command the game vocabulary derives from tick, input, frame and report: rule step, fn summary (src/hop/world.slg), fn read_pad, rule present (src/hop/screen.slg), @on(tick(60Hz)), @window (src/apps/cpp.slg; docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.
#include "game/app/io.hpp"
#include "game/app/program.hpp"
#include "game/cli/exit_code.hpp"
#include "hop/screen/screen.hpp"
#include "hop/world/rules.hpp"

namespace game::app {

int play(hop::levels::LevelNo level, const std::string& assets_dir) {
  auto session = hop::screen::Session::open(hop::screen::Purpose::Play, play_window, assets_dir);
  if (!session) {
    write_stderr("error: " + session.error() + "\n");
    return cli::exit_screen;
  }
  hop::world::World world;
  hop::world::Lines lines = hop::world::Lines::kept();
  hop::world::start(world, level, lines);
  bool written = write_stdout(lines.take());
  while (!session->should_close()) {
    // One tick per frame at a fixed 60 frames per second; a finished game keeps its last frame.
    if (world.mode == hop::model::Mode::Playing && hop::world::step(world, hop::screen::read_pad(), lines)) {
      if (const std::string text = lines.take(); !text.empty()) {
        written = write_stdout(text) && written;
      }
    }
    session->draw(world);
  }
  written = write_stdout(hop::world::summary(world) + "\n") && written;
  if (!written) {
    write_stderr("error: stdout: unwritable\n");
    return cli::exit_output;
  }
  return cli::exit_ok;
}

}  // namespace game::app
