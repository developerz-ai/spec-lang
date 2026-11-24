// The `game` library's runtime for C++ (libs/game/real/cpp): a fixed-tick, single-player game with a window.
// From the app's six fields it derives four commands: `play` (a window, one tick per frame), and `run`, `draw`
// and `shot`, which read a script of controls on standard input.
#pragma once

#include "rt/meta.hpp"

namespace lib::game {

// The process: argv, standard input and the environment in, the two streams and an exit code out.
int main(const rt::meta::App& app, int argc, char** argv);

}  // namespace lib::game
