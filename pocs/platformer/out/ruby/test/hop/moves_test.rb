# frozen_string_literal: true

# Projected by hand from every test of hop.tests.moves (src/hop/tests/moves.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "../test_helper"

module Skyhop
  module TestSupport
    class MovesTest < Minitest::Test
      def started
        s = StartWorlds.session
        s.start(1)
        s
      end

      def test_standing_still_stays_put
        s = started
        30.times { s.step(REST) }

        assert_equal Vec3.new(0, 0, 0), s.player.at
        assert s.player.grounded
        assert_equal 30, s.world.ticks
      end

      def test_running_reaches_full_speed
        s = started
        10.times { s.step(RIGHT) }

        assert_equal 16, s.player.vel.x
        assert_equal 96, s.player.at.x
        assert_equal 0, s.player.yaw
      end

      def test_a_held_jump_peaks_at_380
        s = started
        s.step(REST)
        top = (1..50).map do
          s.step(JUMP)
          s.player.at.y
        end.max

        assert_equal 380, top
        assert s.player.grounded
      end

      def test_a_short_hop_peaks_at_94
        s = started
        s.step(REST)
        s.step(JUMP)
        top = (1..30).map do
          s.step(REST)
          s.player.at.y
        end.max

        assert_equal 94, top
      end

      def test_a_jump_needs_a_fresh_press
        s = started
        s.step(REST)
        60.times { s.step(JUMP) }

        assert s.player.grounded
        assert_equal 0, s.player.at.y
      end

      def test_a_jump_just_after_the_edge_still_counts
        s = StartWorlds.session(:past_the_first_ledge)
        s.step(REST)

        refute s.player.grounded
        s.step(JUMP)

        assert_equal 38, s.player.vel.y
      end

      def test_a_jump_long_after_the_edge_does_not
        s = StartWorlds.session(:past_the_first_ledge)
        7.times { s.step(REST) }
        s.step(JUMP)

        assert_operator s.player.vel.y, :<, 0
      end

      def test_a_wall_stops_the_body_flush
        s = StartWorlds.session(:running_at_the_step)
        10.times { s.step(RIGHT) }

        assert_equal 2496, s.player.at.x
        assert_equal 0, s.player.vel.x
      end

      def test_a_ceiling_stops_a_jump
        s = StartWorlds.session(:under_a_low_ceiling)
        top = (1..40).map do
          s.step(JUMP)
          s.player.at.y
        end.max

        assert_equal 208, top
      end

      def test_a_rider_moves_with_the_mover
        s = StartWorlds.session(:riding_the_mover)
        10.times { s.step(REST) }

        assert_equal Vec3.new(1680, 0, 0), s.player.at
        assert_equal 1, s.player.riding
      end

      def test_a_mover_catches_a_falling_body
        s = StartWorlds.session(:above_the_mover)
        40.times { s.step(REST) }

        assert_equal 1, s.player.riding
        assert_equal 0, s.player.at.y
      end

      def test_a_mover_never_catches_from_below
        s = StartWorlds.session(:under_the_mover)
        s.step(REST)

        assert_operator s.player.at.y, :<, -200
        assert_nil s.player.riding
      end

      def test_a_fast_fall_lands
        s = StartWorlds.session(:falling_fast_onto_the_floor)
        12.times { s.step(REST) }

        assert_equal 0, s.player.at.y
        assert s.player.grounded
      end
    end
  end
end
