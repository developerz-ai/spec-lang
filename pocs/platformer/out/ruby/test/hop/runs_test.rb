# frozen_string_literal: true

# Projected by hand from Hold, pad_of, meadow_win, ridge_win, the_meadow_can_be_won, the_ridge_can_be_won (src/hop/tests/runs.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "../test_helper"

module Skyhop
  module TestSupport
    class RunsTest < Minitest::Test
      Hold = Data.define(:ticks, :keys)

      def self.holds(*pairs) = pairs.each_slice(2).map { |t, k| Hold.new(t, k) }.freeze

      MEADOW_WIN = holds(75, "R", 20, "RJ", 41, "R", 20, "RJ", 35, "R", 20, "RJ", 23, "R", 20, "RJ", 62, "R")
      RIDGE_WIN = holds(
        24, "R", 8, "-", 8, "R", 16, "RJ", 16, "R", 8, "-", 8, "L", 8, "R",
        8, "L", 8, "R", 8, "L", 8, "R", 8, "L", 8, "R", 8, "L", 8, "R",
        16, "RJ", 24, "R", 16, "RJ", 8, "-", 8, "R", 16, "-", 16, "J", 8, "-",
        16, "R", 8, "J", 8, "L", 24, "R", 16, "RJ", 32, "R", 8, "RJ", 8, "J",
        8, "R", 8, "-", 16, "RJ", 32, "R", 16, "RJ", 16, "R", 8, "-", 32, "R",
        16, "RJ", 8, "R"
      )

      def pad_of(keys)
        Pad.new(left: keys.include?("L"), right: keys.include?("R"), forward: keys.include?("F"),
                back: keys.include?("B"), jump: keys.include?("J"))
      end

      def test_pad_of_cases
        assert_equal Pad.new, pad_of("-")
        assert_equal Pad.new(right: true, jump: true), pad_of("RJ")
        assert_equal Pad.new(left: true, forward: true, back: true), pad_of("LFB")
      end

      def test_the_meadow_can_be_won
        s = StartWorlds.session
        s.start(1)
        MEADOW_WIN.each { |h| h.ticks.times { s.step(pad_of(h.keys)) } }

        assert_equal :Won, s.world.mode
        assert_equal 316, s.world.ticks
        assert_equal 3, s.world.taken.size
        assert_equal 3, s.world.lives
      end

      def test_the_ridge_can_be_won
        s = StartWorlds.session
        s.start(2)
        RIDGE_WIN.each { |h| h.ticks.times { s.step(pad_of(h.keys)) if s.world.mode == :Playing } }

        assert_equal :Won, s.world.mode
        assert_equal 549, s.world.ticks
        assert_equal 3, s.world.taken.size
        assert_equal 3, s.world.lives
      end
    end
  end
end
