// Projected by hand from @bin("skyhop"), the app data start, tick, input, frame, dump, report (src/apps/cpp.slg) and the help text of docs/contract.md section 2. Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <string_view>

namespace game::cli {

inline constexpr std::string_view help =
    "usage: skyhop <command> [options]\n"
    "\n"
    "commands:\n"
    "  play [--level <n>]\n"
    "  run [--level <n>]\n"
    "  draw [--level <n>] [--at <tick>]\n"
    "  shot [--level <n>] [--at <tick>] [--out <file.png>]\n"
    "\n"
    "play opens a window: arrows or WASD to run, space to jump, Esc to quit.\n"
    "run, draw and shot read a script of controls on standard input.\n";

}  // namespace game::cli
