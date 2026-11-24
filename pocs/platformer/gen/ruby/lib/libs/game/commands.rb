# frozen_string_literal: true

# The Ruby runtime of the `game` library (libs/game/real/ruby): play, run, draw and shot.
# The same in every app with the game shape; change the library, not this file.

require_relative "script"
require_relative "session"

module Libs
  module Game
    # The four commands. `run`, `draw` and `shot` read the whole script before the first tick and never read
    # the keyboard; only `play` and `shot` open a screen.
    class Commands
      # The frame `shot` draws off screen, in pixels.
      SHOT = [320, 180].freeze

      def initialize(shape, command, libraries:, stdin:, out:, err:, env:)
        @shape = shape
        @command = command
        @screens = libraries.select { |library| library.respond_to?(:open) }
        @stdin = stdin
        @out = out
        @err = err
        @env = env
        @session = Session.new(shape)
      end

      def call
        return play if @command.name == "play"

        holds = Script.read(@shape, @stdin.read)
        case @command.name
        when "run" then run(holds)
        when "draw" then draw(holds)
        else shot(holds)
        end
      rescue Script::Invalid => e
        refuse(e.message, USAGE)
      rescue Spec::Failure => e
        refuse(e.name, REFUSED)
      end

      private

      def refuse(message, code)
        @err.write("error: #{message}\n")
        code
      end

      # `start`, every tick of the script while the game goes on, then the report.
      def run(holds)
        @out.write(@session.start(@command.level), @session.play(holds), @session.report)
        OK
      end

      # The same, silent, up to tick `--at`; then the frame as text.
      def draw(holds)
        @session.start(@command.level)
        @session.play(holds, upto: @command.at)
        @out.write(@session.dump)
        OK
      end

      # The same, then that frame drawn off screen: its pixels hashed, and written as PNG with `--out`.
      def shot(holds)
        require_relative "fnv"
        @session.start(@command.level)
        @session.play(holds, upto: @command.at)
        on_screen(*SHOT, hidden: true) do |window|
          window.capture(*SHOT, -> { @session.frame }) do |pixels, export|
            path = @command.out
            next refuse("--out #{path}: cannot write", WINDOW) if path && !export.call(path)

            @out.write("shot #{FNV.hex(pixels)}\n")
            OK
          end
        end
      end

      # A window, one tick per frame at the tick rate, until the window is closed; then the report.
      def play
        window = @shape.window
        on_screen(window["width"], window["height"], fps: @shape.rate) do |screen|
          @out.write(@session.start(@command.level))
          until screen.closing?
            unless @session.over?
              @out.write(@session.tick(@session.input))
              @out.flush
            end
            screen.frame { @session.frame }
          end
          @out.write(@session.report)
          OK
        end
      end

      # Opens the screen the app's other libraries give, binds its foreign calls, and closes it after the block.
      def on_screen(width, height, hidden: false, fps: nil)
        screen = @screens.first or return refuse("window: cannot open", WINDOW)
        title = @shape.window["title"]
        screen.open(@shape.program, env: @env, prefix: @shape.bin.upcase, width:, height:, title:, hidden:, fps:) do |window|
          @session.bind(window.externs)
          yield window
        end
      rescue screen.unavailable => e
        refuse(e.message, WINDOW)
      end
    end
  end
end
