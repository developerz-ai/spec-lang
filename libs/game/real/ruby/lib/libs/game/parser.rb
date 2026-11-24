# frozen_string_literal: true

# The Ruby runtime of the `game` library (libs/game/real/ruby): the argument parser.
# The same in every app with the game shape; change the library, not this file.

module Libs
  module Game
    # A parsed command: its name, the start rule's argument (Spec::ABSENT: none given), `at` (a tick, or nil for
    # the end of the script) and `out` (a path or nil).
    Command = Data.define(:name, :level, :at, :out)

    # Arguments to a Command, or the first problem from the left.
    module Parser
      # A usage error: the message after "error: ".
      class Usage < StandardError; end

      DIGITS = /\A[0-9]+\z/
      HELP_FLAGS = %w[--help -h].freeze

      module_function

      def parse(shape, argv)
        return :help_to_err if argv.empty?
        return :help if argv.intersect?(HELP_FLAGS)

        name = argv.first
        raise Usage, "unknown command \"#{name}\"" unless shape.flags.key?(name)

        seen = flags(shape, shape.flags.fetch(name), argv.drop(1))
        Command.new(name, seen.fetch(shape.level&.name) { default_level(shape) }, seen["at"], seen["out"])
      end

      def flags(shape, allowed, rest)
        seen = {}
        until rest.empty?
          arg = rest.shift
          raise Usage, "unexpected argument \"#{arg}\"" unless arg.start_with?("--")

          flag = arg.delete_prefix("--")
          raise Usage, "unknown flag #{arg}" unless allowed.include?(flag)
          raise Usage, "#{arg} needs a value" if rest.empty?
          raise Usage, "#{arg} given twice" if seen.key?(flag)

          seen[flag] = value(shape, flag, rest.shift)
        end
        seen
      end

      def value(shape, flag, text)
        return level(shape.level, text) if shape.level && flag == shape.level.name

        case flag
        when "at"
          raise Usage, "--at: not a tick" unless DIGITS.match?(text)

          text.to_i
        else
          raise Usage, "--out: not a path" if text.empty?

          text
        end
      end

      # The start rule's argument: ASCII digits (leading zeros allowed), then the parameter's type.
      def level(param, text)
        value = DIGITS.match?(text) ? text.to_i : nil
        raise Usage, "--#{param.name}: not a #{name_of(param.type)}" unless value && param.type.conforms?(value)

        value
      end

      # Without the flag: the rule's own default, or the least value of a range type.
      def default_level(shape)
        return Spec::ABSENT if shape.level.nil? || shape.level.optional?

        type = shape.level.type
        type = type.kind == :alias ? type.of : type.base while %i[alias where].include?(type.kind)
        type.kind == :range ? type.lo : raise(Usage, "missing --#{shape.level.name}")
      end

      def name_of(type) = type.respond_to?(:name) ? type.name : "Int"
    end
  end
end
