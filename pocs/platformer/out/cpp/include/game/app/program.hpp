// Projected by hand from app SkyhopCpp(start, tick, input, frame, dump, report), @bin("skyhop"), @window(960, 540, "Skyhop") (src/apps/cpp.slg) and the commands of docs/contract.md section 2. Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <optional>
#include <string>
#include <string_view>

#include "game/cli/command.hpp"
#include "hop/screen/session.hpp"

namespace game::app {

// What one invocation leaves behind: the bytes for each stream and the exit code.
struct Outcome {
  int code;
  std::string out;
  std::string err;
  friend bool operator==(const Outcome&, const Outcome&) = default;
};

// @window(960, 540, "Skyhop")
inline const hop::screen::WindowSpec play_window{960, 540, "Skyhop"};
// The off-screen frame of shot: fixed by the contract, with the app's title.
inline const hop::screen::WindowSpec shot_window{320, 180, "Skyhop"};

// start, every tick of the script with its lines, then the summary. No I/O.
Outcome run(hop::levels::LevelNo level, std::string_view input);

// As run, silent, stopping after tick `at`; then the frame as text. No I/O, no window, no asset.
Outcome draw(hop::levels::LevelNo level, std::optional<std::uint32_t> at, std::string_view input);

// As draw, then the frame drawn off screen by raylib and its pixels hashed.
Outcome shot(hop::levels::LevelNo level, std::optional<std::uint32_t> at,
             const std::optional<std::string>& out, std::string_view input, const std::string& assets_dir);

// The window and the 60 Hz loop. Writes standard output itself, flushed every frame.
int play(hop::levels::LevelNo level, const std::string& assets_dir);

// Help, usage errors and dispatch. `input` is called at most once, only by a command that reads
// a script, and only after the arguments were accepted.
int main_program(const cli::Parsed& parsed, std::string (*input)(), const std::string& assets_dir);

}  // namespace game::app
