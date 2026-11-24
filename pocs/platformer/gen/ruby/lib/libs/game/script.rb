# frozen_string_literal: true

# The Ruby runtime of the `game` library (libs/game/real/ruby): the script of controls on standard input.
# The same in every app with the game shape; change the library, not this file.

module Libs
  module Game
    # Reads a script of controls, whole, before the first tick. Each line is `<ticks> <keys>`: hold these
    # controls for that many ticks. `#` starts a comment; `-` holds nothing.
    module Script
      # One line of the script: hold `pad` for `ticks` ticks.
      Hold = Data.define(:ticks, :pad)

      # A bad script: the message after "error: ".
      class Invalid < StandardError; end

      MAX_LINE_TICKS = 100_000
      MAX_TICKS = 1_000_000
      DIGITS = /\A[0-9]+\z/
      FIELD_GAP = /[ \t]+/

      module_function

      # `bytes` is standard input, read whole, as binary. Returns the holds, or raises Invalid.
      def read(shape, bytes)
        holds = []
        bytes.b.split("\n", -1).each_with_index do |line, i|
          hold = line_hold(shape, line.sub(/#.*/m, ""), i + 1)
          holds << hold if hold
        end
        raise Invalid, "input: more than #{MAX_TICKS} ticks" if holds.sum(&:ticks) > MAX_TICKS

        holds
      end

      def line_hold(shape, text, number)
        fields = text.split(FIELD_GAP).reject(&:empty?)
        return nil if fields.empty?
        raise Invalid, "input line #{number}: expected <ticks> <keys>" unless fields.size == 2

        ticks = DIGITS.match?(fields[0]) ? fields[0].to_i : 0
        unless ticks.between?(1, MAX_LINE_TICKS)
          raise Invalid, "input line #{number}: ticks must be 1 to #{MAX_LINE_TICKS}"
        end

        Hold.new(ticks, pad(shape, fields[1], number))
      end

      # Read left to right; the first character that is a problem wins. Characters are UTF-8 code points; each
      # byte of an invalid sequence is a character of its own.
      def pad(shape, field, number)
        held = {}
        unless field == "-"
          field.dup.force_encoding(Encoding::UTF_8).each_char do |c|
            control = shape.keys[c] or raise Invalid, "input line #{number}: unknown key \"#{c.b}\""
            raise Invalid, "input line #{number}: key \"#{c}\" twice" if held.key?(control)

            held[control] = true
          end
        end
        shape.pad.new(*shape.pad.field_names.map { |name| held.key?(name) })
      end
    end
  end
end
