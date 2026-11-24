# frozen_string_literal: true

# Projected by hand from start's parameter level: LevelNo (src/hop/world.slg, src/hop/levels.slg) and the commands of the game vocabulary (docs/game-vocabulary.md). Stand-in for spec-lang build output; change the spec, not this file.

module Skyhop
  module CLI
    # A parsed command: name, level (1 or 2), at (a tick, or nil for the end of the script), out (a path or nil).
    Command = Data.define(:name, :level, :at, :out)

    # Arguments, in the order of contract section 2.
    module Parser
      # A usage error: the message after "error: ".
      class Usage < StandardError; end

      DIGITS = /\A[0-9]+\z/
      HELP_FLAGS = %w[--help -h].freeze
      FLAGS = {
        "play" => %w[level],
        "run" => %w[level],
        "draw" => %w[level at],
        "shot" => %w[level at out]
      }.freeze

      module_function

      def parse(argv)
        return :help_to_err if argv.empty?
        return :help if argv.intersect?(HELP_FLAGS)

        name = argv.first
        raise Usage, "unknown command \"#{name}\"" unless FLAGS.key?(name)

        flags(name, argv.drop(1))
      end

      def flags(name, rest)
        seen = {}
        until rest.empty?
          arg = rest.shift
          raise Usage, "unexpected argument \"#{arg}\"" unless arg.start_with?("--")

          flag = arg.delete_prefix("--")
          raise Usage, "unknown flag #{arg}" unless FLAGS.fetch(name).include?(flag)
          raise Usage, "#{arg} needs a value" if rest.empty?
          raise Usage, "#{arg} given twice" if seen.key?(flag)

          seen[flag] = value(flag, rest.shift)
        end
        Command.new(name, seen.fetch("level", 1), seen["at"], seen["out"])
      end

      def value(flag, text)
        case flag
        when "level"
          raise Usage, "--level: not a LevelNo" unless DIGITS.match?(text) && text.to_i.between?(1, 2)
        when "at" then raise Usage, "--at: not a tick" unless DIGITS.match?(text)
        else raise Usage, "--out: not a path" if text.empty?
        end
        flag == "out" ? text : text.to_i
      end
    end
  end
end
