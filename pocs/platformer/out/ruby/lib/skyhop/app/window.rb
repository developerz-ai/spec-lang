# frozen_string_literal: true

# Projected by hand from @window(960, 540, "Skyhop") on app SkyhopRuby (src/apps/ruby.slg) and the window and asset errors of docs/contract.md section 2. Stand-in for spec-lang build output; change the spec, not this file.

require_relative "../screen"

module Skyhop
  module App
    # Opens raylib and a window, loads the assets, yields them, and always closes what it opened.
    # Errors are exit 3 with one line on standard error and nothing on standard output.
    module Window
      module_function

      def open(env:, err:, flags:, width:, height:)
        return refuse(err, "window: cannot open") unless display?(env)

        Screen::Raylib.open(Screen::Raylib.path(env))
        Screen::Raylib.SetTraceLogLevel(Screen::Raylib::LOG_NONE)
        Screen::Raylib.SetConfigFlags(flags)
        Screen::Raylib.InitWindow(width, height, "Skyhop")
        return refuse(err, "window: cannot open") unless Screen::Raylib.IsWindowReady

        with_assets(env, err) { yield it }
      rescue Screen::Raylib::Missing
        refuse(err, "raylib #{Screen::Raylib.path(env)}: cannot load")
      end

      def with_assets(env, err)
        assets = Screen::Assets.new(Screen::Assets.root(env))
        begin
          yield assets
        ensure
          assets.unload
        end
      rescue Screen::Assets::Unloadable => e
        refuse(err, "asset #{e.message}: cannot load")
      ensure
        Screen::Raylib.CloseWindow
      end

      # raylib 5.5 crashes in InitWindow when there is no display, so it is asked first (contract, No display).
      def display?(env) = %w[DISPLAY WAYLAND_DISPLAY].any? { !env.fetch(it, "").empty? }

      def refuse(err, message)
        err.write("error: #{message}\n")
        CLI::WINDOW
      end
    end
  end
end
