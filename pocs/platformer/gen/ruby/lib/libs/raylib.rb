# frozen_string_literal: true

# The Ruby runtime of the `raylib` library (libs/raylib/real/ruby): the screen a window loop draws on.
# The same in every app that calls raylib; change the library, not this file.

module Libs
  # raylib 5.5 behind the program's `foreign c "raylib#Name"` declarations. Nothing is loaded until a window
  # is asked for: a command that never draws needs neither the library nor the ffi gem.
  module Raylib
    # The window, the library or an asset cannot be had. The message is the line after "error: ".
    class Unavailable < StandardError; end

    # The class a window loop rescues when it asks for a screen.
    def self.unavailable = Unavailable

    # Opens a window, loads the shader and the models, yields the Window, and closes what it opened.
    # `prefix` names the environment variables that say where things are: `<PREFIX>_RAYLIB` (the shared
    # library) and `<PREFIX>_ASSETS` (the folder of `shaders/` and `out/`).
    def self.open(program, env:, prefix:, width:, height:, title:, hidden: false, fps: nil, &)
      # raylib 5.5 crashes in InitWindow when there is no display, so it is asked first.
      raise Unavailable, "window: cannot open" unless %w[DISPLAY WAYLAND_DISPLAY].any? { !env.fetch(it, "").empty? }

      require_relative "raylib/window"
      Window.open(program, env:, prefix:, width:, height:, title:, hidden:, fps:, &)
    end
  end
end
