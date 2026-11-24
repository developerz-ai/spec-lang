# frozen_string_literal: true

# Projected by hand from the state initial values and the named start worlds past_the_first_ledge, running_at_the_step, under_a_low_ceiling, riding_the_mover, above_the_mover, at_the_third_gem, falling_on_the_crawler, running_at_the_crawler, about_to_fall_out, about_to_fall_out_for_the_last_time, beside_the_flag, about_to_fall_out_holding_a_gem, rising_beside_the_crawler, under_the_mover, falling_fast_onto_the_floor (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.

module Skyhop
  module TestSupport
    # The world before any rule runs: every state at the value it declares. `crawlers` and `taken` start empty.
    module StartWorlds
      V = Geom::Vec3
      P = Model::Player
      MEADOW = Levels::MEADOW
      RIDGE = Levels::RIDGE
      INITIAL = World.new(stage: MEADOW, ticks: 0, player: P.new(at: MEADOW.start), taken: Set.new.freeze,
                          crawlers: {}.freeze, lives: 3, mode: :Playing)
      BLACK = Model::Rgb.new(0, 0, 0)
      BOX = Model::Level.new(
        name: "box", start: V.new(0, 0, 0),
        solids: [Model::Solid.new(Geom::Box.new(V.new(-512, -256, -512), V.new(512, 0, 512)), BLACK),
                 Model::Solid.new(Geom::Box.new(V.new(-512, 400, -512), V.new(512, 600, 512)), BLACK)],
        goal: V.new(100_000, 0, 0)
      )
      CRAWLER = { 1 => Model::Crawler.new(1, V.new(3072, 256, 0), 2752, 3392, 1, 2) }.freeze

      WORLDS = {
        past_the_first_ledge: { stage: MEADOW, player: P.new(at: V.new(1090, 0, 0), grounded: true, coyote: 6) },
        running_at_the_step: { stage: MEADOW, player: P.new(at: V.new(2400, 0, 0), vel: V.new(16, 0, 0), grounded: true) },
        under_a_low_ceiling: { stage: BOX, player: P.new(at: V.new(0, 0, 0), grounded: true) },
        riding_the_mover: { stage: RIDGE, player: P.new(at: V.new(1600, 0, 0), grounded: true, riding: 1) },
        above_the_mover: { stage: RIDGE, player: P.new(at: V.new(1600, 200, 0)) },
        at_the_third_gem: { stage: MEADOW, player: P.new(at: V.new(4608, 0, 0), grounded: true) },
        falling_on_the_crawler: { stage: MEADOW, crawlers: CRAWLER,
                                  player: P.new(at: V.new(3072, 400, 0), vel: V.new(0, -20, 0)) },
        running_at_the_crawler: { stage: MEADOW, crawlers: CRAWLER,
                                  player: P.new(at: V.new(2900, 256, 0), vel: V.new(16, 0, 0), grounded: true) },
        about_to_fall_out: { stage: MEADOW, player: P.new(at: V.new(1280, -2040, 0), vel: V.new(0, -48, 0)) },
        about_to_fall_out_for_the_last_time: { stage: MEADOW, lives: 1,
                                               player: P.new(at: V.new(1280, -2040, 0), vel: V.new(0, -48, 0)) },
        beside_the_flag: { stage: MEADOW, player: P.new(at: V.new(5000, 0, 0), grounded: true) },
        about_to_fall_out_holding_a_gem: { stage: MEADOW, taken: Set[1].freeze,
                                           player: P.new(at: V.new(1280, -2040, 0), vel: V.new(0, -48, 0)) },
        rising_beside_the_crawler: { stage: MEADOW, crawlers: CRAWLER,
                                     player: P.new(at: V.new(2920, 330, 0), vel: V.new(0, 20, 0)) },
        under_the_mover: { stage: RIDGE, player: P.new(at: V.new(1600, -200, 0)) },
        falling_fast_onto_the_floor: { stage: MEADOW, player: P.new(at: V.new(0, 400, 0), vel: V.new(0, -48, 0)) }
      }.freeze

      def self.world(name) = INITIAL.with(**WORLDS.fetch(name))

      def self.session(name = nil) = Session.new(name ? world(name) : INITIAL)
    end
  end
end
