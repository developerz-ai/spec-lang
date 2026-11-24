# frozen_string_literal: true

# Projected by hand from every test of hop.tests.touches (src/hop/tests/touches.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "../test_helper"

module Skyhop
  module TestSupport
    class TouchesTest < Minitest::Test
      def test_a_gem_is_taken_once
        s = StartWorlds.session(:at_the_third_gem)
        s.step(REST)

        assert_equal Set[3], s.world.taken
        s.step(REST)

        assert_equal Set[3], s.world.taken
        assert_equal "1 gem 3\n", s.out
      end

      def test_a_fall_keeps_the_gems
        s = StartWorlds.session(:about_to_fall_out_holding_a_gem)
        s.step(REST)

        assert_equal 2, s.world.lives
        assert_equal Set[1], s.world.taken
      end

      def test_rising_into_a_crawler_hurts
        s = StartWorlds.session(:rising_beside_the_crawler)
        s.step(REST)

        assert_equal 2, s.world.lives
        assert s.world.crawlers.key?(1)
      end

      def test_a_stomp_removes_the_crawler
        s = StartWorlds.session(:falling_on_the_crawler)
        s.step(REST)

        refute s.world.crawlers.key?(1)
        assert_equal 24, s.player.vel.y
        assert_equal 3, s.world.lives
      end

      def test_a_crawler_hurts_from_the_side
        s = StartWorlds.session(:running_at_the_crawler)
        s.step(RIGHT)

        assert_equal 2, s.world.lives
        assert_equal Levels::MEADOW.start, s.player.at
        assert s.world.crawlers.key?(1)
      end

      def test_falling_out_costs_a_life
        s = StartWorlds.session(:about_to_fall_out)
        s.step(REST)

        assert_equal 2, s.world.lives
        assert_equal :Playing, s.world.mode
      end

      def test_the_last_life_ends_the_game
        s = StartWorlds.session(:about_to_fall_out_for_the_last_time)
        s.step(REST)

        assert_equal 0, s.world.lives
        assert_equal :Lost, s.world.mode
        assert_raises(World::Over) { s.step(REST) }
      end

      def test_reaching_the_flag_wins
        s = StartWorlds.session(:beside_the_flag)
        s.step(REST)

        assert_equal :Won, s.world.mode
        assert_raises(World::Over) { s.step(REST) }
      end

      def test_starting_again_resets_everything
        s = StartWorlds.session
        s.start(1)
        5.times { s.step(RIGHT) }
        s.start(2)

        assert_empty s.world.taken
        assert_equal 0, s.world.ticks
        assert_equal 2, s.world.crawlers.size
        assert_equal Levels::RIDGE.start, s.player.at
      end
    end
  end
end
