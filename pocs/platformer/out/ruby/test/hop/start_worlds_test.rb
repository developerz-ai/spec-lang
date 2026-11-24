# frozen_string_literal: true

# Projected by hand from the named start worlds, each a `test ... = {}` (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "../test_helper"

module Skyhop
  module TestSupport
    # Each start world is a test: its @given builds, and the world is the one it describes.
    class StartWorldsTest < Minitest::Test
      StartWorlds::WORLDS.each do |name, given|
        define_method(:"test_#{name}") do
          world = StartWorlds.world(name)

          given.each { |state, value| assert_equal value, world.public_send(state) }
          assert_equal 0, world.ticks
          assert_equal :Playing, world.mode
        end
      end

      def test_every_start_world_of_the_spec_is_here
        names = %i[past_the_first_ledge running_at_the_step under_a_low_ceiling riding_the_mover above_the_mover
                   at_the_third_gem falling_on_the_crawler running_at_the_crawler about_to_fall_out
                   about_to_fall_out_for_the_last_time beside_the_flag about_to_fall_out_holding_a_gem
                   rising_beside_the_crawler under_the_mover falling_fast_onto_the_floor]

        assert_equal names, StartWorlds::WORLDS.keys
      end
    end
  end
end
