# frozen_string_literal: true

# The Ruby runtime of the `cli` library (libs/cli/real/ruby): the command table, derived from the rules.
# The same in every app that uses `cli`; change the library, not this file.

require_relative "arguments"

module Libs
  module Cli
    # A parameter without a default: a positional argument, shown `<name>`.
    Positional = Data.define(:index, :name, :type) do
      def shown = "<#{name}>"
    end

    # A parameter with a default: a flag `--name`. `kind` is :switch (`Bool`), :option (`Option[T]`, one value at
    # most), :set (`Set[T]`, repeatable) or :once (any other type, one value at most). `type` is the value's.
    Flag = Data.define(:index, :name, :kind, :type) do
      def shown = "--#{name}"

      def usage
        case kind
        when :switch then "[#{shown}]"
        when :set then "[#{shown} #{Arguments.placeholder(type)}]..."
        else "[#{shown} #{Arguments.placeholder(type)}]"
        end
      end
    end

    # One rule with `@on(cli(name))`.
    Command = Data.define(:name, :rule, :positionals, :flags) do
      def self.of(name, rule)
        positionals = []
        flags = []
        rule.params.each_with_index do |param, index|
          if param.optional? then flags << flag(index, param)
          else positionals << Positional.new(index:, name: param.name, type: param.type)
          end
        end
        new(name:, rule:, positionals:, flags:)
      end

      def self.flag(index, param)
        type = param.type
        kind = { bool: :switch, option: :option, set: :set }.fetch(type.kind, :once)
        Flag.new(index:, name: param.name, kind:, type: %i[option set].include?(kind) ? type.of : type)
      end

      def flag(shown) = flags.find { |flag| flag.shown == shown }

      def usage = [name, *positionals.map(&:shown), *flags.map(&:usage)].join(" ")
    end

    # Every command of an app, in the order its `rules: [...]` lists them, and the help they make.
    class Table
      attr_reader :commands

      def initialize(program, app)
        @bin = app.note("bin")["name"]
        rules = app.params.fetch("rules", []).map { |name| program.rules.fetch(name) }
        @commands = rules.filter_map do |rule|
          trigger = rule.triggers.find { |note| note.library == "cli" && note.name == "cli" }
          trigger && Command.of(trigger["command"], rule)
        end
      end

      def find(name) = commands.find { |command| command.name == name }

      # `--help`: one line per command.
      def help
        "usage: #{@bin} <command> [args]\n\ncommands:\n#{commands.map { |command| "  #{command.usage}\n" }.join}"
      end
    end
  end
end
