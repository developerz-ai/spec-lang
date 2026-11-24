# frozen_string_literal: true

# The Ruby runtime of the `game` library (libs/game/real/ruby): one run of the command line.
# The same in every app with the game shape; change the library, not this file.

require_relative "game/shape"
require_relative "game/parser"

module Libs
  # A fixed-tick, single-player game with a window: `play` in a window, and `run`, `draw` and `shot` over a
  # script of controls on standard input. Everything is read from the app's manifest (`PROGRAM.apps`): the six
  # declarations its shape names, `@bin`, `@window`, and the `tick` trigger's rate. Drawing and the keyboard are
  # another library's: the one among `libraries` that opens a screen.
  module Game
    OK = 0
    REFUSED = 1
    USAGE = 2
    WINDOW = 3

    def self.run(program, app, argv, stdin:, out:, err:, env:, libraries: [])
      shape = Shape.new(program, app)
      command = Parser.parse(shape, argv.map(&:b))
      case command
      when :help_to_err then err.write(shape.help) && USAGE
      when :help then out.write(shape.help) && OK
      else
        require_relative "game/commands"
        Commands.new(shape, command, libraries:, stdin:, out:, err:, env:).call
      end
    rescue Parser::Usage => e
      err.write("error: #{e.message}\n")
      USAGE
    end
  end
end
