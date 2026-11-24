require_relative "json_reader"
require_relative "../values"
require_relative "../text"

module Runtime
  module Store
    # One type's place in the store file: a strict reader (`decode`, from the JSON tree) and the
    # canonical writer (`encode`, appending to a buffer).
    module Codec
      # The JSON is valid but is not a world of this program.
      class Malformed < StandardError; end

      # The store does not hold this value: an Int past 2^53. Reported as `full`.
      class Full < StandardError; end

      # The largest magnitude of a stored Int: a backend constraint carried as a refusal, not a language rule.
      MAX_INT = 2**53

      # `fast` reads the exact bytes `encode` writes from a StringScanner, with no JSON tree. It never
      # decides that a store is unreadable: on anything else it throws :slow, and the strict reader runs.
      Pair = Data.define(:decode, :encode, :fast)

      def self.slow = throw(:slow)

      INT = /-?(?:0|[1-9][0-9]{0,15})/
      # A JSON string holding nothing but raw characters and the two escapes the writer emits.
      PLAIN_STRING = /"(?:[^"\\\u0000-\u001F]|\\["\\])*"/
      UNESCAPE = /\\(["\\])/

      def self.malformed(what) = raise(Malformed, what)

      # An Int: a JSON number with no fraction or exponent, within 2^53, then the type's claim.
      def self.int(type = nil)
        Pair.new(
          decode: lambda do |json|
            malformed("integer") unless json.is_a?(Integer) && json.abs <= MAX_INT && (type.nil? || type.valid?(json))
            json
          end,
          encode: lambda do |value, out|
            raise Full if value.abs > MAX_INT

            out << value.to_s
          end,
          fast: lambda do |scanner|
            raw = scanner.scan(INT) or slow
            value = raw.to_i
            slow unless value.abs <= MAX_INT && (type.nil? || type.valid?(value))
            value
          end
        )
      end

      NEEDS_ESCAPE = /["\\\u0000-\u001F]/

      # Only `"`, `\` and what JSON cannot hold raw are escaped; everything else is raw UTF-8.
      def self.quote(text, out)
        out << '"'
        out << if text.match?(NEEDS_ESCAPE)
                 text.gsub(NEEDS_ESCAPE) { |c| ['"', "\\"].include?(c) ? "\\#{c}" : format("\\u%04x", c.ord) }
               else
                 text
               end
        out << '"'
      end

      # A Text, then the type's claim.
      def self.text(type = nil)
        Pair.new(
          decode: lambda do |json|
            malformed("text") unless Text.text?(json) && (type.nil? || type.valid?(json))
            json.frozen? ? json : json.dup.freeze
          end,
          encode: ->(value, out) { quote(value, out) },
          fast: lambda do |scanner|
            raw = scanner.scan(PLAIN_STRING) or slow
            body = raw[1...-1]
            body = body.gsub(UNESCAPE, "\\1") if body.include?("\\")
            slow unless Text.text?(body) && (type.nil? || type.valid?(body))
            body.freeze
          end
        )
      end

      def self.bool
        Pair.new(
          decode: ->(json) { [true, false].include?(json) ? json : malformed("boolean") },
          encode: ->(value, out) { out << value.to_s },
          fast: ->(scanner) { (raw = scanner.scan(/true|false/)) ? raw == "true" : slow }
        )
      end

      # A sum with no data: the variant's name.
      def self.variant(all)
        by_name = all.to_h { |variant| [variant.name, variant] }.freeze
        Pair.new(
          decode: ->(json) { (json.is_a?(String) && by_name[json]) || malformed("variant") },
          encode: ->(value, out) { out << '"' << value.name << '"' },
          fast: lambda do |scanner|
            raw = scanner.scan(PLAIN_STRING) or slow
            by_name[raw[1...-1]] || slow
          end
        )
      end

      def self.sequence(items, out, of)
        out << "["
        items.each_with_index do |item, index|
          out << "," unless index.zero?
          of.encode.call(item, out)
        end
        out << "]"
      end

      # The items of a canonical array: no white space, and strictly ascending by `key`, which is the
      # only form the writer emits and rules out repeats.
      def self.fast_items(scanner, of)
        scanner.skip("[") or slow
        items = []
        last = nil
        until scanner.skip("]")
          scanner.skip(",") or slow unless items.empty?
          item = of.fast.call(scanner)
          order = block_given? ? yield(item) : item
          slow unless last.nil? || (last <=> order)&.negative?
          last = order
          items << item
        end
        items
      end

      # A Set: an array in any order with no value twice; written ascending.
      def self.set(of)
        Pair.new(
          decode: lambda do |json|
            malformed("array") unless json.is_a?(Array)
            items = json.map { |item| of.decode.call(item) }
            malformed("value twice") unless items.uniq.size == items.size
            items.to_set.freeze
          end,
          encode: ->(value, out) { sequence(value.sort, out, of) },
          fast: ->(scanner) { fast_items(scanner, of).to_set.freeze }
        )
      end

      def self.fields_of(json, fields, extra)
        malformed("keys") unless json.is_a?(Hash) && json.size == fields.size + extra
        fields.to_h do |key, codec|
          [key, codec.decode.call(json.fetch(key) { malformed("no #{key}") })]
        end
      end

      def self.members(fields, values, out)
        fields.each_with_index do |(key, codec), index|
          out << "," unless index.zero?
          out << '"' << key << '":'
          codec.encode.call(values.fetch(key), out)
        end
      end

      # The members of a canonical object, in the writer's order. `lead` is what stands before the first.
      def self.fast_fields(scanner, fields, lead)
        values = {}
        fields.each do |key, codec|
          scanner.skip(%(#{lead}"#{key}":)) or slow
          values[key] = codec.fast.call(scanner)
          lead = ","
        end
        values
      end

      # A record: an object with exactly these keys, written in this order.
      def self.record(fields, get:, make:)
        Pair.new(
          decode: lambda do |json|
            make.call(fields_of(json, fields, 0))
          rescue ClaimViolation => e
            malformed(e.message)
          end,
          encode: lambda do |value, out|
            out << "{"
            members(fields, get.call(value), out)
            out << "}"
          end,
          fast: lambda do |scanner|
            scanner.skip("{") or slow
            value = make.call(fast_fields(scanner, fields, ""))
            scanner.skip("}") or slow
            value
          end
        )
      end

      # A Map whose values hold their own key: an array of them, no key twice; written ascending by key.
      def self.keyed(of, &key)
        Pair.new(
          decode: lambda do |json|
            malformed("array") unless json.is_a?(Array)
            map = {}
            json.each do |item|
              value = of.decode.call(item)
              malformed("key twice") if map.key?(key.call(value))
              map[key.call(value)] = value
            end
            ascending = map.each_key.each_cons(2).all? { |a, b| (a <=> b).negative? }
            ascending ? map : map.sort.to_h
          end,
          encode: ->(value, out) { sequence(value.values, out, of) },
          fast: ->(scanner) { fast_items(scanner, of, &key).to_h { |value| [key.call(value), value] } }
        )
      end

      # The whole store file: `{"version":N, ...states}` on one line. `laws` are the program's laws of the
      # form "in every state, p": a store that breaks one was not written by these rules, so it is refused.
      Whole = Data.define(:version, :fields, :get, :make, :laws, :speculate) do
        # Store text (tagged UTF-8, not yet checked) to the world.
        # Strategy store.canonical-fast-path: this program wrote the store, so its exact bytes are read first.
        def decode(text)
          world = (speculate && text.valid_encoding? && catch(:slow) { fast(text) }) || strict(text)
          Codec.malformed("law") unless laws.all? { |law| law.call(world) }
          world
        rescue ClaimViolation => e
          raise Malformed, e.message
        end

        def fast(text)
          scanner = StringScanner.new(text)
          scanner.skip(%({"version":#{version})) or Codec.slow
          world = make.call(Codec.fast_fields(scanner, fields, ","))
          scanner.skip("}\n") && scanner.eos? ? world : Codec.slow
        end

        def strict(text)
          json = JsonReader.parse(text)
          Codec.malformed("version") unless json.is_a?(Hash) && json["version"] == version
          make.call(Codec.fields_of(json, fields, 1))
        rescue JsonReader::Invalid => e
          raise Malformed, e.message
        end

        # The world to its one canonical line.
        # `capacity` sizes the buffer once: the bytes read, when a store was read, are close to the bytes written.
        def encode(world, capacity: 256)
          out = String.new(capacity:, encoding: Encoding::UTF_8)
          out << '{"version":' << version.to_s
          values = get.call(world)
          fields.each do |key, codec|
            out << ',"' << key << '":'
            codec.encode.call(values.fetch(key), out)
          end
          out << "}\n"
        end
      end

      def self.store(version, fields, get:, make:, laws:, speculate:)
        Whole.new(version:, fields:, get:, make:, laws:, speculate:)
      end
    end
  end
end
