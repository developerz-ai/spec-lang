# frozen_string_literal: true

# The Ruby runtime of the `cli` library (libs/cli/real/ruby): a strict JSON reader.
# The same in every app that uses `cli`; change the library, not this file.

require "strscan"

module Libs
  module Cli
    module Store
      # A strict JSON reader. Unlike JSON.parse it refuses a repeated key and a lone surrogate, and it never
      # rounds: a number with no fraction or exponent is an exact Integer, any other number is a Real.
      class JsonReader
        class Invalid < StandardError; end

        # A number that is not an integer literal, kept as written. `1.0` and `1e0` land here.
        Real = Data.define(:literal)

        # The store nests four deep. The bound keeps hostile input from exhausting the stack.
        MAX_DEPTH = 16
        SPACE = /[ \t\n\r]*/
        NUMBER = /-?(?:0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?/
        PLAIN = /[^"\\\u0000-\u001F]+/
        SIMPLE_STRING = /"[^"\\\u0000-\u001F]*"/
        OPEN_OBJECT = "{".ord
        OPEN_ARRAY = "[".ord
        QUOTE = '"'.ord
        # Punctuation with the white space around it, so that each is one scan.
        OPEN = /[{\[][ \t\n\r]*/
        EMPTY_OBJECT = /\{[ \t\n\r]*\}/
        EMPTY_ARRAY = /\[[ \t\n\r]*\]/
        COLON = /[ \t\n\r]*:/
        COMMA = /[ \t\n\r]*,[ \t\n\r]*/
        CLOSE_OBJECT = /[ \t\n\r]*\}/
        CLOSE_ARRAY = /[ \t\n\r]*\]/
        HEX4 = /[0-9a-fA-F]{4}/
        ESCAPES = { '"' => '"', "\\" => "\\", "/" => "/", "b" => "\b", "f" => "\f", "n" => "\n", "r" => "\r",
                    "t" => "\t" }.freeze
        LITERALS = { "true" => true, "false" => false, "null" => nil }.freeze
        LITERAL = /true|false|null/
        HIGH_SURROGATES = (0xD800..0xDBFF)
        LOW_SURROGATES = (0xDC00..0xDFFF)

        def self.parse(text) = new(text).parse

        def initialize(text)
          raise Invalid, "not UTF-8" unless text.encoding == Encoding::UTF_8 && text.valid_encoding?

          @scanner = StringScanner.new(text)
        end

        def parse
          value = value(0)
          @scanner.skip(SPACE)
          raise Invalid, "text after the value" unless @scanner.eos?

          value
        end

        private

        def value(depth)
          raise Invalid, "nested too deep" if depth > MAX_DEPTH

          @scanner.skip(SPACE)
          case @scanner.peek_byte
          when OPEN_OBJECT then object(depth + 1)
          when OPEN_ARRAY then array(depth + 1)
          when QUOTE then string
          else scalar
          end
        end

        def object(depth)
          members = {}
          return members if @scanner.skip(EMPTY_OBJECT)

          @scanner.skip(OPEN)
          loop do
            key = string
            raise Invalid, "key #{key} twice" if members.key?(key)

            expect(COLON)
            members[key] = value(depth)
            next if @scanner.skip(COMMA)

            expect(CLOSE_OBJECT)
            return members
          end
        end

        def array(depth)
          items = []
          return items if @scanner.skip(EMPTY_ARRAY)

          @scanner.skip(OPEN)
          loop do
            items << value(depth)
            next if @scanner.skip(COMMA)

            expect(CLOSE_ARRAY)
            return items
          end
        end

        def scalar
          number = @scanner.scan(NUMBER)
          return @scanner[1] || @scanner[2] ? Real.new(literal: number) : number.to_i if number

          literal = @scanner.scan(LITERAL) or raise Invalid, "not a value at byte #{@scanner.pos}"
          LITERALS.fetch(literal)
        end

        # Most strings hold no escape and are one scan and one slice.
        def string
          simple = @scanner.scan(SIMPLE_STRING)
          return simple.byteslice(1, simple.bytesize - 2).freeze if simple

          expect(/"/)
          text = +""
          loop do
            plain = @scanner.scan(PLAIN)
            text << plain if plain
            return text.freeze if @scanner.skip(/"/)

            expect(/\\/)
            text << escape
          end
        end

        def escape
          char = @scanner.getch
          return [code_point].pack("U") if char == "u"

          ESCAPES.fetch(char) { raise Invalid, "bad escape at byte #{@scanner.pos}" }
        end

        def code_point
          unit = hex4
          return unit unless HIGH_SURROGATES.cover?(unit) || LOW_SURROGATES.cover?(unit)
          raise Invalid, "lone surrogate" unless HIGH_SURROGATES.cover?(unit) && @scanner.skip(/\\u/)

          low = hex4
          raise Invalid, "lone surrogate" unless LOW_SURROGATES.cover?(low)

          0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00)
        end

        def hex4
          digits = @scanner.scan(HEX4) or raise Invalid, "bad \\u escape at byte #{@scanner.pos}"
          digits.to_i(16)
        end

        def expect(pattern)
          @scanner.skip(pattern) or raise Invalid, "expected #{pattern.source} at byte #{@scanner.pos}"
        end
      end
    end
  end
end
