# frozen_string_literal: true

# The runtime of the Ruby printer of spec-lang build: the values that are not plain Ruby values.
# The same in every generated project; change the printer, not this file.
#
# Bool, Int, Text, List, Set and Map are true/false, Integer, a UTF-8 String, an Array, a Set and a Hash; a Set and
# a Hash are kept in ascending order of the value order (spec/order.rb), so they iterate as the language says.
# `()` is nil. A range is a Range of Integers. A function is a lambda.

module Spec
  # `Some(value)`. `none` is NONE, never nil: `Some(none)` and `none` differ.
  Some = Data.define(:value) do
    # `option else fallback`.
    def or = value
  end

  # The class of `none`.
  class None
    def or = yield

    def inspect = "none"
  end

  NONE = None.new.freeze

  # A trace step's reply: `Ok(value)` or `Err(failure)`.
  Ok = Data.define(:value)
  Err = Data.define(:error)

  # `(a, b)`: an Array that shows and conforms as a tuple.
  class Tuple < Array; end

  # What a record class and a sum's module share with Spec::Type: a value checked against the type.
  module Declared
    # The value, or E-CLAIM: a value outside its type reached a place of that type.
    def check(value, what = "a value")
      reason = reject(value)
      Spec.stop!("E-CLAIM", "#{what}: #{reason}") if reason
      value
    end

    def conforms?(value) = reject(value).nil?
  end

  # What a record class is extended with: its name and field types, for claims, show, order and the libraries.
  module Record
    include Declared

    # `spec "Task", %w[no title]` when a member had to be renamed for Ruby; else the members are the names.
    def spec(name, field_names = nil, &types)
      @spec_name = name
      @field_names = field_names
      @types = types
    end

    def spec_name = @spec_name

    def kind = :record

    def field_names = @field_names ||= members.map(&:to_s)

    def field_types = @field_types ||= @types.call

    # [name, member, type] per field, in declared order.
    def fields = @fields ||= field_names.zip(members, field_types)

    def strict?
      return @strict unless @strict.nil?

      @strict = false # a type that holds itself: no claim found on the way round
      @strict = field_types.any?(&:strict?)
    end

    def strict_fields = @strict_fields ||= fields.select { |_, _, type| type.strict? }

    # Raises E-CLAIM when a field breaks its declared type. Every constructor calls it.
    def claim!(record)
      strict_fields.each do |name, member, type|
        reason = type.reject(record.public_send(member))
        Spec.stop!("E-CLAIM", "field `#{name}`: #{reason}") if reason
      end
    end

    def reject(value)
      return "#{Spec.show(value)} is not a #{spec_name}" unless value.is_a?(self)

      strict_fields.each do |name, member, type|
        reason = type.reject(value.public_send(member))
        return "field `#{name}`: #{reason}" if reason
      end
      nil
    end

  end

  # What a sum's module is extended with: `Open = variant("Open")`, `Held = variant("Held", :by) { [TEXT] }`.
  module Sum
    include Declared

    def spec(name) = @spec_name = name

    def spec_name = @spec_name

    def kind = :sum

    def variants = @variants ||= []

    # A variant without fields is one frozen value; one with fields is a Data class.
    def variant(name, *members, field_names: nil, &types)
      made = members.empty? ? Constant.new(self, variants.size, name) : fielded(name, members, field_names, types)
      variants << made
      made
    end

    def strict?
      return @strict unless @strict.nil?

      @strict = false
      @strict = variants.any? { |v| v.is_a?(Class) && v.field_types.any?(&:strict?) }
    end

    def reject(value)
      return "#{Spec.show(value)} is not a #{spec_name}" unless value.respond_to?(:spec_sum) && value.spec_sum.equal?(self)

      value.is_a?(Constant) ? nil : value.class.reject_fields(value)
    end

    private

    def fielded(name, members, field_names, types)
      sum = self
      tag = variants.size
      Data.define(*members) do
        extend Fields
        include Fielded

        setup(sum, tag, name, field_names, types)
      end
    end

    # A variant without fields.
    class Constant
      attr_reader :spec_sum, :spec_tag, :variant_name

      def initialize(sum, tag, name)
        @spec_sum = sum
        @spec_tag = tag
        @variant_name = name
        freeze
      end

      def spec_fields = []

      def inspect = variant_name
    end

    # Class side of a variant with fields.
    module Fields
      attr_reader :spec_sum, :spec_tag, :variant_name

      def setup(sum, tag, name, field_names, types)
        @spec_sum = sum
        @spec_tag = tag
        @variant_name = name
        @field_names = field_names
        @types = types
      end

      def field_names = @field_names ||= members.map(&:to_s)

      def field_types = @field_types ||= @types.call

      def strict_fields
        @strict_fields ||= field_names.zip(members, field_types).select { |_, _, type| type.strict? }
      end

      def reject_fields(value)
        strict_fields.each do |name, member, type|
          reason = type.reject(value.public_send(member))
          return "field `#{name}`: #{reason}" if reason
        end
        nil
      end

      def claim!(value)
        reason = reject_fields(value)
        Spec.stop!("E-CLAIM", reason) if reason
      end
    end

    # Instance side of a variant with fields.
    module Fielded
      def spec_sum = self.class.spec_sum

      def spec_tag = self.class.spec_tag

      def variant_name = self.class.variant_name

      def spec_fields = deconstruct
    end
  end

  # A call of a rule as a trace records it. `rule` is the program's Spec::Rule.
  Event = Data.define(:rule, :args) do
    # `s.event.no`: the argument of that name; a rule without it stops.
    def arg(name)
      index = rule.params.index { |param| param.name == name }
      Spec.stop!("E-TYPE-MISMATCH", "the event has no argument `#{name}`") if index.nil?
      args.fetch(index)
    end

    def call_of?(qualified) = rule.name == qualified
  end

  # One step of a trace: the call, its reply, the world before and after.
  Step = Data.define(:event, :reply, :before, :after)

  # A trace: its steps, and the start world followed by the world after each step.
  Trace = Data.define(:steps, :states)
end
