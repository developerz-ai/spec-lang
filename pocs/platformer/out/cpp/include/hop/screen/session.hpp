// Projected by hand from the window, the assets and the off-screen frame of the game vocabulary (docs/game-vocabulary.md; docs/contract.md sections 2 shot, 2 play and 4) for app SkyhopCpp (src/apps/cpp.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <cstdint>
#include <expected>
#include <optional>
#include <string>
#include <vector>

#include "hop/world/world.hpp"

namespace hop::screen {

// What the window is opened for.
enum class Purpose {
  Play,  // visible, 4x MSAA hint, 60 frames per second
  Shot,  // hidden; the frame is drawn off screen
};

struct WindowSpec {
  int width;
  int height;
  std::string title;
};

// The frame `shot` drew: width * height * 4 bytes, R8G8B8A8, as LoadImageFromTexture returns them.
struct Capture {
  std::vector<std::uint8_t> rgba;
  bool exported;  // false only when a PNG was asked for and ExportImage refused
};

// Whether DISPLAY or WAYLAND_DISPLAY names a display (contract section 2, "No display").
bool has_display();

// An open raylib window with the shader and the four models loaded. One at a time (raylib has one
// window). Closing unloads the assets in reverse order, then closes the window.
class Session {
 public:
  // The error is the message of contract section 2 without "error: ": "window: cannot open" or
  // "asset <path>: cannot load".
  static std::expected<Session, std::string> open(Purpose purpose, const WindowSpec& window,
                                                  const std::string& assets_dir);
  Session(Session&& other) noexcept;
  Session& operator=(Session&&) = delete;
  Session(const Session&) = delete;
  Session& operator=(const Session&) = delete;
  ~Session();

  // Esc or the close button.
  bool should_close() const;
  // BeginDrawing, present, EndDrawing.
  void draw(const world::World& world);
  // Draws the frame into a render texture of the given size and reads it back; with a path, also
  // writes it there as PNG.
  Capture capture(const world::World& world, int width, int height, const std::optional<std::string>& png);

 private:
  Session() = default;
  bool open_ = true;
};

}  // namespace hop::screen
