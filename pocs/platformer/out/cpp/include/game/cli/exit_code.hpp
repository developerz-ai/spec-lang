// Projected by hand from the exit codes of the game vocabulary (docs/contract.md section 2) for app SkyhopCpp (src/apps/cpp.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

namespace game::cli {

inline constexpr int exit_ok = 0;
// Not in the contract: standard output could not be written (see NOTES.md, open choices).
inline constexpr int exit_output = 1;
inline constexpr int exit_usage = 2;   // usage or input
inline constexpr int exit_screen = 3;  // the window or an asset cannot be opened

}  // namespace game::cli
