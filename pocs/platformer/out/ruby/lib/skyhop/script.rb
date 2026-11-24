# frozen_string_literal: true

# Projected by hand from the input script the game vocabulary derives from step's parameter Pad(left, right, forward, back, jump) (docs/game-vocabulary.md, src/hop/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "model"

module Skyhop
  # Reads a script of controls, whole, before the first tick (contract section 2, the input script).
  module Script
    # One line of the script: hold `pad` for `ticks` ticks.
    Hold = Data.define(:ticks, :pad)

    # A bad script: the message after "error: ".
    class Invalid < StandardError; end

    MAX_LINE_TICKS = 100_000
    MAX_TICKS = 1_000_000
    # One letter per Pad field, its first letter in upper case.
    KEYS = { "L" => :left, "R" => :right, "F" => :forward, "B" => :back, "J" => :jump }.freeze
    DIGITS = /\A[0-9]+\z/
    FIELD_GAP = /[ \t]+/

    module_function

    # `bytes` is standard input, read whole, as binary. Returns the holds, or raises Invalid.
    def read(bytes)
      holds = []
      bytes.b.split("\n", -1).each_with_index do |line, i|
        hold = line_hold(line.sub(/#.*/m, ""), i + 1)
        holds << hold if hold
      end
      raise Invalid, "input: more than #{MAX_TICKS} ticks" if holds.sum(&:ticks) > MAX_TICKS

      holds
    end

    def line_hold(text, number)
      fields = text.split(FIELD_GAP).reject(&:empty?)
      return nil if fields.empty?
      raise Invalid, "input line #{number}: expected <ticks> <keys>" unless fields.size == 2

      ticks = ticks_of(fields[0])
      raise Invalid, "input line #{number}: ticks must be 1 to #{MAX_LINE_TICKS}" unless ticks

      Hold.new(ticks, pad_of(fields[1], number))
    end

    def ticks_of(field)
      return nil unless DIGITS.match?(field)

      n = field.to_i
      n if n.between?(1, MAX_LINE_TICKS)
    end

    # Read left to right; the first character that is a problem wins. Characters are UTF-8 code points; each
    # byte of an invalid sequence is a character of its own.
    def pad_of(field, number)
      return Model::Pad.new if field == "-"

      held = {}
      field.dup.force_encoding(Encoding::UTF_8).each_char do |c|
        raise Invalid, "input line #{number}: unknown key \"#{c.b}\"" unless KEYS.key?(c)
        raise Invalid, "input line #{number}: key \"#{c}\" twice" if held.key?(KEYS[c])

        held[KEYS[c]] = true
      end
      Model::Pad.new(**held)
    end
  end
end
