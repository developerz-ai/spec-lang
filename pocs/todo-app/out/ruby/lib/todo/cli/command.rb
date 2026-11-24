# frozen_string_literal: true

# Projected by hand from the derivation of arguments from a rule's parameters (docs/cli-vocabulary.md, "Derived, never written"). Stand-in for spec-lang build output; change the spec, not this file.

module Todo
  module CLI
    # A parameter without a default: a positional argument, shown `<name>`.
    Positional = Data.define(:name, :type) do
      def shown = "<#{name}>"
    end

    # A parameter with a default. `kind` is :switch (Bool = false), :once (one value at most)
    # or :set (Set[T], repeatable).
    Flag = Data.define(:name, :kind, :type, :default) do
      def shown = "--#{name}"

      def usage
        case kind
        when :switch then "[#{shown}]"
        when :once then "[#{shown} #{type::PLACEHOLDER}]"
        when :set then "[#{shown} #{type::PLACEHOLDER}]..."
        end
      end
    end

    # One `@on(cli(name))` rule. `access` is :write or :read, from the rule's `@uses`;
    # `reads` is :world, or :counts when the rule reads no more than that.
    Command = Data.define(:name, :rule, :access, :reads, :positionals, :flags) do
      def flag(shown) = flags.find { |flag| flag.shown == shown }

      def defaults = flags.to_h { |flag| [flag.name, flag.default] }

      def usage = [name, *positionals.map(&:shown), *flags.map(&:usage)].join(" ")
    end
  end
end
