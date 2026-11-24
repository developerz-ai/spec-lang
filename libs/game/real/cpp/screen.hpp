// The `game` library's runtime for C++ (libs/game/real/cpp): the window, as the game loop needs it. `game`
// draws nothing itself: the library whose calls state `screen.write` (raylib among them) installs a Screen.
#pragma once

#include <cstdint>
#include <functional>
#include <optional>
#include <string>
#include <vector>

#include "rt/meta.hpp"

namespace lib::game {

// What the window is opened for.
enum class Purpose {
  Play,  // visible, at the tick rate
  Shot,  // hidden; one frame is drawn off screen
};

struct Window {
  int width = 0;
  int height = 0;
  std::string title;
  // Frames per second of `play`: the tick rate.
  int rate = 60;
};

// One frame drawn off screen: width * height * 4 bytes, R8G8B8A8, top row first.
struct Capture {
  std::vector<std::uint8_t> rgba;
  // False only when a file was asked for and could not be written.
  bool exported = true;
};

class Screen {
 public:
  virtual ~Screen() = default;
  // Opens the window and loads what the app's drawing calls need from `assets_dir`. An error is its
  // message without "error: ": "window: cannot open", "asset <path>: cannot load".
  virtual std::optional<std::string> open(Purpose purpose, const Window& window, const rt::meta::App& app,
                                          const std::string& assets_dir) = 0;
  // The player asked to leave.
  virtual bool should_close() = 0;
  // One frame on the window: `frame` makes the app's drawing calls.
  virtual void draw(const std::function<void()>& frame) = 0;
  // One frame off screen, read back; with a path, also written there as PNG.
  virtual Capture capture(int width, int height, const std::function<void()>& frame,
                          const std::optional<std::string>& png) = 0;
  // Unloads in reverse order and closes the window.
  virtual void close() = 0;
};

// The one screen of the process; null until a drawing library installs its own.
Screen* screen();
void install_screen(Screen* screen);

}  // namespace lib::game
