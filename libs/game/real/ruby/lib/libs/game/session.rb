# frozen_string_literal: true

# The Ruby runtime of the `game` library (libs/game/real/ruby): one game being played.
# The same in every app with the game shape; change the library, not this file.

module Libs
  module Game
    # The real `out` handler: lines are kept until the rule that sent them commits.
    class Output
      def initialize = @lines = []

      def perform(operation, args)
        raise ArgumentError, "out.#{operation} is for tests" unless operation == :line

        @lines << args.fetch(0)
        nil
      end

      def mark = @lines.size

      def rewind(mark) = @lines.slice!(mark..)

      # The lines sent since the last take, each ending in a newline.
      def take = @lines.slice!(0..).map { |line| "#{line}\n" }.join
    end

    # One world, from `start` on. Every call gives the lines the program sent with `out.line`.
    class Session
      attr_reader :ticks

      def initialize(shape)
        @shape = shape
        @machine = Spec::Machine.new(shape.program)
        @output = Output.new
        shape.program.handlers.each_key { |key| @machine.serve(key, @output) if key.end_with?(".out") }
        @ticks = 0
        @over = false
      end

      # Foreign calls a screen serves, by `library#symbol`.
      def bind(externs) = @machine.bind(externs)

      def start(level)
        on { @shape.start.call(@shape.level ? [level] : []) }
        @output.take
      end

      def over? = @over

      # One tick with these controls. A refusal ends the run: no later tick is made.
      def tick(pad)
        on { @shape.tick.call([pad]) }
        @ticks += 1
        @output.take
      rescue Spec::Failure
        @over = true
        ""
      end

      # Every tick of the script while the game goes on, up to tick `upto` (nil: the end of the script).
      def play(holds, upto: nil)
        text = +""
        holds.each do |hold|
          hold.ticks.times do
            return text if over? || (upto && ticks >= upto)

            text << tick(hold.pad)
          end
        end
        text
      end

      def input = on { @shape.input.call([]) }

      def frame = on { @shape.frame.call([]) }

      def dump = on { @shape.dump.call([]) }.map { |line| "#{line}\n" }.join

      def report = "#{on { @shape.report.call([]) }}\n"

      private

      def on(&) = Spec.on(@machine, &)
    end
  end
end
