# frozen_string_literal: true

# Projected by hand from test touch_matches_raylib(a: Box, b: Box) (src/hop/tests/edge.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "../test_helper"
require_relative "../../lib/skyhop/screen"

module Skyhop
  module TestSupport
    # The game's touch against raylib's CheckCollisionBoxes, through the real library: 2,000 box pairs with
    # coordinates drawn from Sub, half of them built so that b starts within one sub-metre of a's far faces.
    class TouchMatchesRaylibTest < Minitest::Test
      PAIRS = 2_000
      SUB = (-1_048_576..1_048_576)
      SEED = Integer(ENV.fetch("SKYHOP_SEED", "20261124"))

      def setup = Screen::Raylib.open(Screen::Raylib.path(ENV))

      def test_touch_matches_raylib
        rng = Random.new(SEED)
        touching = 0
        PAIRS.times do |i|
          a = box(rng)
          b = i.even? ? near(a, rng) : box(rng)
          touching += 1 if Geom.touches(a, b)

          assert_equal Geom.touches(a, b), Screen.check_collision_boxes(a, b), "pair #{i}: #{a} #{b}"
        end
        assert_operator touching, :>, PAIRS / 10, "too few pairs touch to test the faces"
      end

      def box(rng)
        lo = Vec3.new(*Array.new(3) { rng.rand(SUB.min..(SUB.max - 1)) })
        hi = Vec3.new(rng.rand(lo.x..SUB.max), rng.rand(lo.y..SUB.max), rng.rand(lo.z..SUB.max))
        Box.new(lo, hi)
      end

      # On each axis b starts at a's far face, one before or one past it, or overlaps a; size from 0 to 512.
      def near(a, rng)
        lo = %i[x y z].map do |axis|
          edge = rng.rand(2).zero? ? a.hi.public_send(axis) : a.lo.public_send(axis) - rng.rand(0..512)
          (edge + rng.rand(-1..1)).clamp(SUB.min, SUB.max)
        end
        hi = lo.map { (it + rng.rand(0..512)).clamp(it, SUB.max) }
        Box.new(Vec3.new(*lo), Vec3.new(*hi))
      end
    end
  end
end
