# frozen_string_literal: true

# Projected by hand from every @cases of plus, around_feet, overlaps, touches, triangle, approach, vec_text (src/hop/geom.slg), q (src/hop/levels.slg), walk_target, facing, patrol (src/hop/physics.slg), asset_name, bob, draw_text (src/hop/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "../test_helper"

module Skyhop
  module TestSupport
    # One table per @cases: each row is (arguments, expected).
    class CasesTest < Minitest::Test
      def self.box(lo, hi) = Box.new(Vec3.new(*lo), Vec3.new(*hi))

      def self.crawler(x, dir) = Model::Crawler.new(1, Vec3.new(x, 0, 0), -10, 10, dir, 4)

      TABLES = {
        [Geom, :plus] => [[[Vec3.new(1, 2, 3), Vec3.new(10, 20, 30)], Vec3.new(11, 22, 33)]],
        [Geom, :around_feet] => [[[Vec3.new(0, 0, 0), Vec3.new(64, 192, 64)], box([-64, 0, -64], [64, 192, 64])]],
        [Geom, :overlaps] => [
          [[box([0, 0, 0], [2, 2, 2]), box([1, 1, 1], [3, 3, 3])], true],
          [[box([0, 0, 0], [2, 2, 2]), box([2, 0, 0], [4, 2, 2])], false]
        ],
        [Geom, :touches] => [
          [[box([0, 0, 0], [2, 2, 2]), box([2, 0, 0], [4, 2, 2])], true],
          [[box([0, 0, 0], [2, 2, 2]), box([3, 0, 0], [4, 2, 2])], false]
        ],
        [Geom, :triangle] => [[[0, 4], 0], [[3, 4], 3], [[4, 4], 4], [[5, 4], 3], [[8, 4], 0], [[9, 4], 1]],
        [Geom, :approach] => [[[0, 3, 2], 2], [[5, 3, 2], 3], [[5, 0, 2], 3], [[-5, 0, 1], -4]],
        [Geom, :vec_text] => [[[Vec3.new(1, -2, 3)], "1,-2,3"]],
        [Levels, :q] => [[[4], 256], [[-2], -128]],
        [Physics, :walk_target] => [
          [[Pad.new], Vec3.new(0, 0, 0)],
          [[Pad.new(right: true)], Vec3.new(16, 0, 0)],
          [[Pad.new(left: true, forward: true)], Vec3.new(-11, 0, -11)],
          [[Pad.new(left: true, right: true)], Vec3.new(0, 0, 0)]
        ],
        [Physics, :facing] => [
          [[Pad.new, 90], 90], [[Pad.new(right: true), 90], 0],
          [[Pad.new(forward: true), 0], 90], [[Pad.new(left: true, back: true), 0], 225]
        ],
        [Physics, :patrol] => [
          [[crawler(0, 1)], crawler(4, 1)],
          [[crawler(8, 1)], crawler(10, -1)]
        ],
        [Render, :asset_name] => [[[:PipMesh], "pip"], [[:CrawlerMesh], "crawler"], [[:GemMesh], "gem"],
                                  [[:FlagMesh], "flag"]],
        [Render, :bob] => [[[0], -32], [[16], 0], [[32], 32], [[48], 0]],
        [Render, :draw_text] => [
          [[Render::Cube.new(box([0, 0, 0], [64, 64, 64]), Model::Rgb.new(1, 2, 3))], "cube 0,0,0 64,64,64 1,2,3"],
          [[Render::Mesh.new(:PipMesh, Vec3.new(1, 2, 3), 90)], "mesh pip 1,2,3 90"]
        ]
      }.freeze

      TABLES.each do |(mod, name), rows|
        define_method(:"test_#{name}_cases") do
          rows.each { |args, expected| assert_equal expected, mod.public_send(name, *args), "#{name}#{args}" }
        end
      end
    end
  end
end
