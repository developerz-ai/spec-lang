# frozen_string_literal: true

# Projected by hand from the test world of todo.tests.scenarios: rules called directly, `out.text`, `-> value`, `-> fail Err` (src/todo/tests/scenarios.slg, docs/cli-vocabulary.md). Stand-in for spec-lang build output; change the spec, not this file.

module Todo
  module TestSupport
    # A world that rules are called on directly, and everything `out.line` sent. Not a mock: the rules are the real ones.
    class BoardSession
      attr_reader :world, :out_text

      def initialize(world = Board::World.initial)
        @world = world
        @out_text = +""
      end

      # `board.rule(args)`: commits the new world, sends the lines, gives the reply.
      def call(rule, **args)
        outcome = Board.public_send(rule, @world, **args)
        @world = outcome.world
        outcome.lines.each { |line| @out_text << line << "\n" }
        outcome.reply
      end
    end
  end
end
