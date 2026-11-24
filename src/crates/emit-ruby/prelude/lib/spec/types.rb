# frozen_string_literal: true

# The runtime of the Ruby printer of spec-lang build: declared types as values.
# The same in every generated project; change the printer, not this file.

module Spec
  # A declared type: what a value is checked against (`check`), what a generator draws from and what a library's
  # runtime reads to parse an argument or keep a store. Record classes and sum modules answer the same questions
  # (spec/values.rb). Types are made once and compared by identity.
  class Type
    attr_reader :kind

    def initialize(kind) = @kind = kind

    # Whether a value of the right shape can still be outside the type: a range or a claim somewhere inside.
    def strict? = false

    # nil when the value is of the type, else why it is not.
    def reject(_value) = nil

    # The value, or E-CLAIM: a value outside its type reached a place of that type.
    def check(value, what = "a value")
      reason = reject(value)
      Spec.stop!("E-CLAIM", "#{what}: #{reason}") if reason
      value
    end

    def conforms?(value) = reject(value).nil?

    # `Bool`, `Int`, `Text`, `()`.
    class Base < Type
      def initialize(kind, name, ruby)
        super(kind)
        @name = name
        @ruby = ruby
      end

      def reject(value) = @ruby.any? { |c| c === value } ? nil : "#{Spec.show(value)} is not #{@name}"
    end

    UNIT = Base.new(:unit, "()", [nil])
    BOOL = Base.new(:bool, "a Bool", [true, false])
    INT = Base.new(:int, "an Int", [Integer])
    TEXT = Base.new(:text, "Text", [String])

    # `lo..=hi` or `lo..<hi` as a type.
    class Range < Type
      attr_reader :lo, :hi, :inclusive

      def initialize(lo, hi, inclusive)
        super(:range)
        @lo = lo
        @hi = hi
        @inclusive = inclusive
      end

      def strict? = true

      # The greatest member.
      def last = inclusive ? hi : hi - 1

      def reject(value)
        return nil if value.is_a?(Integer) && value >= lo && value <= last

        "#{Spec.show(value)} is not in #{lo}#{inclusive ? "..=" : "..<"}#{hi}"
      end
    end

    # `Option[T]`, `List[T]`, `Set[T]`.
    class Of < Type
      attr_reader :of

      def initialize(kind, of)
        super(kind)
        @of = of
      end

      def strict? = of.strict?

      def reject(value)
        return nil unless strict?

        case kind
        when :option then value.is_a?(Some) ? of.reject(value.value) : nil
        else first_reason(value, of)
        end
      end

      def first_reason(items, type)
        items.each do |item|
          reason = type.reject(item)
          return reason if reason
        end
        nil
      end
    end

    # `Map[K, V]`: the keys are checked first, then the values.
    class Map < Of
      attr_reader :key

      def initialize(key, of)
        super(:map, of)
        @key = key
      end

      def strict? = key.strict? || of.strict?

      def reject(value)
        return nil unless strict?

        first_reason(value.keys, key) || first_reason(value.values, of)
      end
    end

    # `(A, B)`.
    class TupleOf < Type
      attr_reader :items

      def initialize(items)
        super(:tuple)
        @items = items
      end

      def strict? = items.any?(&:strict?)

      def reject(value)
        items.each_with_index do |type, index|
          reason = type.reject(value[index])
          return reason if reason
        end
        nil
      end
    end

    # `T where claim`: the base first, then the claim. True holds; false or a failure does not.
    class Where < Type
      attr_reader :text

      def initialize(base, text, claim)
        super(:where)
        @base = base
        @text = text
        @claim = claim
      end

      def base = @base.call

      def strict? = true

      def reject(value)
        reason = base.reject(value)
        return reason if reason

        @claim.call(value) == true ? nil : "`#{text}` is false for #{Spec.show(value)}"
      rescue Failure => e
        "the claim failed with #{e.name}"
      end
    end

    # `type Title = Text where ...`: a name for another type. A value of it is a value of `of` that conforms.
    class Alias < Type
      attr_reader :name

      def initialize(name, of)
        super(:alias)
        @name = name
        @body = of
      end

      def of = @of ||= @body.call

      def strict?
        return @strict unless @strict.nil?

        @strict = false
        @strict = of.strict?
      end

      def reject(value) = of.reject(value)
    end

    # `type Change = a | b;`: the calls of these rules.
    class Rules < Type
      attr_reader :name, :rules

      def initialize(name, rules)
        super(:rules)
        @name = name
        @rules = rules
      end

      def strict? = true

      def reject(value)
        value.is_a?(Event) && rules.include?(value.rule.name) ? nil : "#{Spec.show(value)} is not a call of one of its rules"
      end
    end

    TRACE = new(:trace)

    @made = {}

    class << self
      def range(lo, hi, exclusive: false) = made([:range, lo, hi, exclusive]) { Range.new(lo, hi, !exclusive) }

      def option(of) = made([:option, of]) { Of.new(:option, of) }

      def list(of) = made([:list, of]) { Of.new(:list, of) }

      def set(of) = made([:set, of]) { Of.new(:set, of) }

      def map(key, of) = made([:map, key, of]) { Map.new(key, of) }

      def tuple(*items) = made([:tuple, *items]) { TupleOf.new(items) }

      # `id` is the claim's number in the program: one claim, one type, wherever it is named.
      def where(id, text, base, &claim) = made([:where, id]) { Where.new(base, text, claim) }

      def alias(name, &of) = Alias.new(name, of)

      def rules(name, *rules) = Rules.new(name, rules)

      private

      def made(key) = @made[key] ||= yield
    end
  end
end
