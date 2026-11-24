# frozen_string_literal: true

# Projected by hand from the play command the game vocabulary derives from tick, input and frame, and @on(tick(60Hz)) on step (docs/game-vocabulary.md, src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "window"

module Skyhop
  module App
    # A window, one tick per frame at 60 frames a second, until Esc or the close button.
    module Play
      module_function

      def call(command, out:, err:, env:)
        Window.open(env:, err:, flags: Screen::Raylib::FLAG_MSAA_4X_HINT, width: 960, height: 540) do |assets|
          Screen::Raylib.SetTargetFPS(60)
          world = loop_frames(World.start(command.level), Screen::Presenter.new(assets), out)
          out.write(world.summary, "\n")
          CLI::OK
        end
      end

      def loop_frames(started, presenter, out)
        out.write(App.lines(started.lines))
        world = started.world
        until Screen::Raylib.WindowShouldClose
          if world.mode == :Playing
            outcome = world.step(Screen.read_pad)
            out.write(App.lines(outcome.lines))
            out.flush
            world = outcome.world
          end
          Screen::Raylib.BeginDrawing
          presenter.present(world)
          Screen::Raylib.EndDrawing
        end
        world
      end
    end
  end
end
