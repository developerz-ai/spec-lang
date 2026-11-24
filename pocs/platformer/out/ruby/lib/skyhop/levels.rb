# frozen_string_literal: true

# Projected by hand from LevelNo, grass, stone, wood, q, at, slab, meadow, ridge, level (src/hop/levels.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "model"

module Skyhop
  # The two levels, written in quarter metres. Built once at load: every value here is a constant.
  module Levels
    include Model

    GRASS = Rgb.new(98, 170, 80)
    STONE = Rgb.new(150, 150, 160)
    WOOD = Rgb.new(210, 150, 60)

    def self.q(n) = n * 64

    def self.at(x, y, z) = Geom::Vec3.new(q(x), q(y), q(z))

    def self.slab(from, to, color) = Solid.new(Geom::Box.new(from, to), color)

    MEADOW = Level.new(
      name: "meadow",
      start: at(0, 0, 0),
      solids: [
        slab(at(-8, -4, -8), at(16, 0, 8), GRASS),
        slab(at(24, -4, -8), at(40, 0, 8), GRASS),
        slab(at(40, -4, -8), at(56, 4, 8), STONE),
        slab(at(64, -4, -8), at(84, 0, 8), GRASS)
      ],
      gems: [Gem.new(1, at(32, 6, 0)), Gem.new(2, at(60, 10, 0)), Gem.new(3, at(72, 2, 0))],
      crawlers: [Crawler.new(1, at(48, 4, 0), q(43), q(53), 1, 2)],
      goal: at(80, 0, 0)
    )

    RIDGE = Level.new(
      name: "ridge",
      start: at(0, 0, 0),
      solids: [
        slab(at(-8, -4, -8), at(16, 0, 8), GRASS),
        slab(at(52, -4, -8), at(60, 0, 8), GRASS),
        slab(at(60, -4, -8), at(76, 5, 8), STONE),
        slab(at(80, -4, -8), at(100, 5, 8), GRASS)
      ],
      movers: [Mover.new(1, Geom::Box.new(at(20, -2, -6), at(32, 0, 6)), Geom::Vec3.new(8, 0, 0), 128, WOOD)],
      gems: [Gem.new(1, at(34, 5, 0)), Gem.new(2, at(68, 9, 0)), Gem.new(3, at(90, 9, 0))],
      crawlers: [
        Crawler.new(1, at(68, 5, 0), q(62), q(74), 1, 3),
        Crawler.new(2, at(88, 5, 0), q(82), q(94), -1, 2)
      ],
      goal: at(96, 5, 0)
    )

    LEVEL_NOS = (1..2)

    def self.level(n) = n == 1 ? MEADOW : RIDGE
  end
end
