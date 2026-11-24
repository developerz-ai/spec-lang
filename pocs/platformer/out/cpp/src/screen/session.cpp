// Projected by hand from the window, the assets and the off-screen frame of the game vocabulary (docs/game-vocabulary.md; docs/contract.md sections 2 shot, 2 play and 4) for app SkyhopCpp (src/apps/cpp.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "hop/screen/session.hpp"

#include <cstddef>
#include <cstdlib>

#include "assets.hpp"
#include "hop/screen/screen.hpp"
#include "raylib.h"

namespace hop::screen {

bool has_display() {
  const auto set = [](const char* name) {
    const char* const value = std::getenv(name);
    return value != nullptr && *value != '\0';
  };
  return set("DISPLAY") || set("WAYLAND_DISPLAY");
}

std::expected<Session, std::string> Session::open(Purpose purpose, const WindowSpec& window,
                                                  const std::string& assets_dir) {
  // raylib 5.5 crashes in InitWindow when there is no display, so that case is refused first.
  if (!has_display()) {
    return std::unexpected(std::string("window: cannot open"));
  }
  SetTraceLogLevel(LOG_NONE);
  SetConfigFlags(purpose == Purpose::Shot ? FLAG_WINDOW_HIDDEN : FLAG_MSAA_4X_HINT);
  InitWindow(window.width, window.height, window.title.c_str());
  if (!IsWindowReady()) {
    return std::unexpected(std::string("window: cannot open"));
  }
  if (auto assets = load_assets(assets_dir); !assets) {
    CloseWindow();
    return std::unexpected(std::move(assets.error()));
  }
  if (purpose == Purpose::Play) {
    SetTargetFPS(60);
  }
  return Session();
}

Session::Session(Session&& other) noexcept : open_(other.open_) { other.open_ = false; }

Session::~Session() {
  if (open_) {
    unload_assets();
    CloseWindow();
  }
}

bool Session::should_close() const { return WindowShouldClose(); }

void Session::draw(const world::World& world) {
  BeginDrawing();
  present(world);
  EndDrawing();
}

Capture Session::capture(const world::World& world, int width, int height,
                         const std::optional<std::string>& png) {
  const RenderTexture2D target = LoadRenderTexture(width, height);
  BeginTextureMode(target);
  present(world);
  EndTextureMode();
  Image image = LoadImageFromTexture(target.texture);
  // A render texture reads back bottom row first; flipped, the hash and the PNG are top row first.
  ImageFlipVertical(&image);

  Capture shot{{}, true};
  if (image.data != nullptr) {
    const auto* bytes = static_cast<const std::uint8_t*>(image.data);
    shot.rgba.assign(
        bytes, bytes + static_cast<std::size_t>(image.width) * static_cast<std::size_t>(image.height) * 4);
  }
  if (png) {
    shot.exported = ExportImage(image, png->c_str());
  }
  UnloadImage(image);
  UnloadRenderTexture(target);
  return shot;
}

}  // namespace hop::screen
