// Projected by hand from Asset, asset_name (src/hop/render.slg) and load_assets (docs/contract.md section 4). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <array>
#include <expected>
#include <string>

#include "raylib.h"

namespace hop::screen {

struct Assets {
  Shader lit;
  int time_loc;
  std::array<Model, 4> models;  // indexed by render::Asset
};

// After the window opens: the lit shader, then each Asset in declared order with the shader on
// every material. On failure everything already loaded is unloaded again and the error is
// "asset <path>: cannot load".
std::expected<void, std::string> load_assets(const std::string& dir);

// In reverse order of loading. Before CloseWindow.
void unload_assets();

// The assets of the open session.
const Assets& assets();

}  // namespace hop::screen
