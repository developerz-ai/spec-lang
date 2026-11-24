# frozen_string_literal: true

# Projected by hand from Pad, Rgb, Solid, Mover, Gem, Crawler, Level, Mode, Player (src/hop/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "geom"

module Skyhop
  # What a level and the bodies in it are. Data only.
  module Model
    ORIGIN = Geom::Vec3.new(0, 0, 0)

    Pad = Data.define(:left, :right, :forward, :back, :jump) do
      def initialize(left: false, right: false, forward: false, back: false, jump: false) = super
    end

    Rgb = Data.define(:r, :g, :b)
    Solid = Data.define(:box, :color)
    Mover = Data.define(:no, :home, :step, :span, :color)
    Gem = Data.define(:no, :at)
    Crawler = Data.define(:no, :at, :lo, :hi, :dir, :speed)

    Level = Data.define(:name, :start, :solids, :movers, :gems, :crawlers, :goal) do
      def initialize(name:, start:, solids:, goal:, movers: [], gems: [], crawlers: []) = super
    end

    # Mode is the symbol of its variant: :Playing, :Won or :Lost.
    MODES = %i[Playing Won Lost].freeze

    # `riding` is the mover's number, or nil for `none`.
    Player = Data.define(:at, :vel, :grounded, :coyote, :jump_held, :riding, :yaw) do
      def initialize(at:, vel: ORIGIN, grounded: false, coyote: 0, jump_held: false, riding: nil, yaw: 0) = super
    end
  end
end
