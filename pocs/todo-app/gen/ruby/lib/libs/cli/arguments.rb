# frozen_string_literal: true

# The Ruby runtime of the `cli` library (libs/cli/real/ruby): an argument's text to a value of a declared type.
# The same in every app that uses `cli`; change the library, not this file.

module Libs
  module Cli
    # The arguments are not a call of any rule. The message is the line after "error: ".
    class UsageError < StandardError; end

    # A parameter's type has no command-line form (a record, a List, a Map).
    class NoForm < StandardError; end

    # One argument's text as a value of a declared type: an Int is ASCII digits (leading zeros allowed, any
    # length), a Text is the text as given, a sum without data is a variant's name in lower case; then the
    # type's claim. A value outside the type is no value.
    module Arguments
      DIGITS = /\A-?[0-9]+\z/

      module_function

      # [value], or nil when the text is not of the type.
      def parse(type, text)
        return nil unless text.valid_encoding?

        found = read(type, text)
        found && type.conforms?(found.first) ? found : nil
      end

      def read(type, text)
        case type.kind
        when :alias then read(type.of, text)
        when :where then read(type.base, text)
        when :int, :range then text.match?(DIGITS) ? [Integer(text, 10)] : nil
        when :text then [text.frozen? ? text : text.dup.freeze]
        when :sum
          variant = words(type)[text]
          variant && [variant]
        else raise NoForm, "a #{type.kind} has no command-line form"
        end
      end

      # A sum's variants by their lower-case names; only a sum with no data has a form.
      def words(type)
        raise NoForm, "a sum with data has no command-line form" if type.variants.any?(Class)

        type.variants.to_h { |variant| [variant.variant_name.downcase, variant] }
      end

      # The type's name in an error: `--tags: not a Tag`.
      def name(type)
        case type.kind
        when :alias then type.name
        when :where then name(type.base)
        when :sum, :record then type.spec_name
        when :option, :set, :list then name(type.of)
        when :int, :range then "Int"
        when :text then "Text"
        else type.kind.to_s.capitalize
        end
      end

      # What stands for a value in the help: `low|normal|high` for a sum, else `<tag>`.
      def placeholder(type)
        base = type
        base = base.kind == :alias ? base.of : base.base while %i[alias where].include?(base.kind)
        base.kind == :sum ? words(base).keys.join("|") : "<#{name(type).downcase}>"
      end
    end
  end
end
