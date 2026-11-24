// Projected by hand from Asset, asset_name (src/hop/render.slg) and load_assets (docs/contract.md section 4). Stand-in for spec-lang build output; change the spec, not this file.
#include "assets.hpp"

#include <cstddef>

#include "hop/render/frame.hpp"
#include "hop/render/text.hpp"

namespace hop::screen {
namespace {

// raylib has one window and one GL context, so the loaded assets are one value for the process.
Assets loaded{};
std::size_t models_loaded = 0;
bool shader_loaded = false;

}  // namespace

std::expected<void, std::string> load_assets(const std::string& dir) {
  const std::string vertex = dir + "/shaders/lit.vs";
  const std::string fragment = dir + "/shaders/lit.fs";
  loaded.lit = LoadShader(vertex.c_str(), fragment.c_str());
  shader_loaded = true;
  if (!IsShaderValid(loaded.lit)) {
    unload_assets();
    return std::unexpected("asset " + vertex + ": cannot load");
  }
  loaded.time_loc = GetShaderLocation(loaded.lit, "uTime");

  for (const render::Asset asset : render::assets) {
    std::string path = dir + "/out/";
    path += render::asset_name(asset);
    path += ".glb";
    Model& model = loaded.models[static_cast<std::size_t>(asset)];
    model = LoadModel(path.c_str());
    ++models_loaded;
    if (!IsModelValid(model)) {
      unload_assets();
      return std::unexpected("asset " + path + ": cannot load");
    }
    // raylib draws a model with its materials' shader; BeginShaderMode does not reach DrawModelEx.
    for (int i = 0; i < model.materialCount; ++i) {
      model.materials[i].shader = loaded.lit;
    }
  }
  return {};
}

void unload_assets() {
  while (models_loaded > 0) {
    --models_loaded;
    Model& model = loaded.models[models_loaded];
    // UnloadModel frees the meshes and the material maps and leaves the shared shader alone.
    UnloadModel(model);
  }
  if (shader_loaded) {
    UnloadShader(loaded.lit);
    shader_loaded = false;
  }
  loaded = {};
}

const Assets& assets() { return loaded; }

}  // namespace hop::screen
