# frozen_string_literal: true

# The Ruby runtime of the `cli` library (libs/cli/real/ruby): one declared type's place in the store.
# The same in every app that uses `cli`; change the library, not this file.

module Libs
  module Cli
    module Store
      # The JSON is valid but is not a world of this program.
      class Malformed < StandardError; end

      # The store does not hold this value: an Int past 2^53. Reported as `full`.
      class TooLarge < StandardError; end

      # The world has no form in the store: a Map kept by its values holds one under another key.
      class Unkeepable < StandardError; end

      # A value of a declared type as JSON and back: a strict reader (`decode`, from the JSON tree) and the
      # canonical writer (`encode`, appending to a buffer: no white space, a Set and a Map ascending).
      #
      # | Type | JSON |
      # | Int, a range | a number with no fraction or exponent, within 2^53 |
      # | Text, Bool, () | a string (only `"` and `\` escaped), true or false, null |
      # | a sum | a variant without data is its name; one with data is `{"Name": {field: value}}` |
      # | a record | an object with exactly its fields, in declared order |
      # | Option | null, or the value (`{"some": value}` where the value could itself be null) |
      # | List, Set, tuple | an array; a Set holds no value twice |
      # | Map | an array of its values when they are records whose first field is of the key's type and holds |
      # |     | the key; else an array of `[key, value]`; no key twice |
      module Codec
        # The largest magnitude of a stored Int: a constraint of the store carried as a refusal, not a language rule.
        MAX_INT = 2**53
        NEEDS_ESCAPE = /["\\\u0000-\u001F]/

        module_function

        def malformed(what) = raise(Malformed, what)

        # The type under its names and claims.
        def base(type)
          type = type.kind == :alias ? type.of : type.base while %i[alias where].include?(type.kind)
          type
        end

        def encode(type, value, out)
          type = base(type)
          case type.kind
          when :int, :range then out << integer(value)
          when :text then quote(value, out)
          when :bool then out << value.to_s
          when :unit then out << "null"
          when :sum then encode_variant(value, out)
          when :record then encode_fields(type.fields, value, out)
          when :option then encode_option(type, value, out)
          when :list, :set then sequence(value.to_a, out) { |item| encode(type.of, item, out) }
          when :tuple then sequence(value.each_index.to_a, out) { |i| encode(type.items[i], value[i], out) }
          when :map then encode_map(type, value, out)
          else raise ArgumentError, "a #{type.kind} cannot be kept in a store"
          end
        end

        def decode(type, json)
          value = decode_base(base(type), json)
          malformed("claim") unless type.conforms?(value)
          value
        end

        def decode_base(type, json)
          case type.kind
          when :int, :range then json.is_a?(Integer) && json.abs <= MAX_INT ? json : malformed("integer")
          when :text then json.is_a?(String) ? json : malformed("text")
          when :bool then [true, false].include?(json) ? json : malformed("boolean")
          when :unit then json.nil? ? nil : malformed("null")
          when :sum then decode_variant(type, json)
          when :record then type.new(*decode_fields(type.fields, json))
          when :option then decode_option(type, json)
          when :list then array(json).map { |item| decode(type.of, item) }
          when :set then distinct(array(json).map { |item| decode(type.of, item) })
          when :tuple then decode_tuple(type, json)
          when :map then decode_map(type, json)
          else raise ArgumentError, "a #{type.kind} cannot be kept in a store"
          end
        end

        def integer(value)
          raise TooLarge if value.abs > MAX_INT

          value.to_s
        end

        # Only `"`, `\` and what JSON cannot hold raw are escaped; everything else is raw UTF-8.
        def quote(text, out)
          out << '"'
          out << if text.match?(NEEDS_ESCAPE)
                   text.gsub(NEEDS_ESCAPE) { |c| ['"', "\\"].include?(c) ? "\\#{c}" : format("\\u%04x", c.ord) }
                 else
                   text
                 end
          out << '"'
        end

        def sequence(items, out)
          out << "["
          items.each_with_index do |item, index|
            out << "," unless index.zero?
            yield item
          end
          out << "]"
        end

        def array(json) = json.is_a?(Array) ? json : malformed("array")

        def distinct(items)
          malformed("value twice") unless items.uniq.size == items.size
          Spec::Sets.of(items)
        end

        def encode_fields(fields, value, out)
          out << "{"
          fields.each_with_index do |(name, member, type), index|
            out << "," unless index.zero?
            out << '"' << name << '":'
            encode(type, value.public_send(member), out)
          end
          out << "}"
        end

        def decode_fields(fields, json)
          malformed("keys") unless json.is_a?(Hash) && json.size == fields.size
          fields.map { |name, _, type| decode(type, json.fetch(name) { malformed("no #{name}") }) }
        end

        def variant_fields(variant) = variant.field_names.zip(variant.members, variant.field_types)

        def encode_variant(value, out)
          return out << '"' << value.variant_name << '"' if value.spec_fields.empty?

          out << '{"' << value.variant_name << '":'
          encode_fields(variant_fields(value.class), value, out)
          out << "}"
        end

        def decode_variant(type, json)
          name, fields = json.is_a?(Hash) && json.size == 1 ? json.first : [json, nil]
          variant = type.variants.find { |v| v.variant_name == name } or malformed("variant")
          malformed("variant") unless variant.is_a?(Class) == !fields.nil?
          fields.nil? ? variant : variant.new(*decode_fields(variant_fields(variant), fields))
        end

        # Whether `none` and `Some(value)` could both be null.
        def wrapped?(type) = %i[option unit].include?(base(type.of).kind)

        def encode_option(type, value, out)
          return out << "null" if value.equal?(Spec::NONE)
          return encode(type.of, value.value, out) unless wrapped?(type)

          out << '{"some":'
          encode(type.of, value.value, out)
          out << "}"
        end

        def decode_option(type, json)
          return Spec::NONE if json.nil?
          return Spec::Some.new(decode(type.of, json)) unless wrapped?(type)

          malformed("option") unless json.is_a?(Hash) && json.keys == ["some"]
          Spec::Some.new(decode(type.of, json["some"]))
        end

        def decode_tuple(type, json)
          malformed("tuple") unless array(json).size == type.items.size
          Spec::Tuple.new(type.items.zip(json).map { |item, value| decode(item, value) })
        end

        # A Map whose values hold their own key: records whose first field is declared with the key's type.
        def keyed?(type)
          value = base(type.of)
          value.kind == :record && !value.fields.empty? && value.fields.first[2].equal?(type.key)
        end

        def encode_map(type, value, out)
          unless keyed?(type)
            return sequence(value.to_a, out) do |key, item|
              out << "["
              encode(type.key, key, out)
              out << ","
              encode(type.of, item, out)
              out << "]"
            end
          end
          member = base(type.of).fields.first[1]
          sequence(value.to_a, out) do |key, item|
            raise Unkeepable, "a value is stored under another key" unless item.public_send(member) == key

            encode(type.of, item, out)
          end
        end

        def decode_map(type, json)
          member = keyed?(type) ? base(type.of).fields.first[1] : nil
          pairs = array(json).map do |item|
            next pair(type, item) unless member

            value = decode(type.of, item)
            [value.public_send(member), value]
          end
          malformed("key twice") unless pairs.map(&:first).uniq.size == pairs.size
          Spec::Maps.of(pairs)
        end

        def pair(type, json)
          malformed("pair") unless array(json).size == 2
          [decode(type.key, json[0]), decode(type.of, json[1])]
        end
      end
    end
  end
end
