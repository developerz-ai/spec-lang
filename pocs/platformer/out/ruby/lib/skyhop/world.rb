# frozen_string_literal: true

# Projected by hand from the states stage, ticks, player, taken, crawlers, lives, mode and from start, lose, summary, View, view (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "levels"
require_relative "physics"

module Skyhop
  # The seven states as one immutable value. A rule takes a world and returns an Outcome with a new one,
  # so a refusal (raised before any new world exists) leaves the world unchanged, as rule atomicity asks.
  # `taken` is a frozen Set; `crawlers` a frozen Hash from number to Crawler, in ascending key order.
  World = Data.define(:stage, :ticks, :player, :taken, :crawlers, :lives, :mode)

  class World
    # The guard `mode == Playing else Over` of `lose` and `step`.
    class Over < StandardError
      def initialize(msg = "Over") = super
    end

    # A committed rule: the new world and the lines it sent to `out.line`, in order.
    Outcome = Data.define(:world, :lines)

    View = Data.define(:stage, :ticks, :player, :taken, :crawlers, :lives, :mode)

    MODE_TEXT = { Playing: "playing", Won: "won", Lost: "lost" }.freeze
    NO_GEMS = Set.new.freeze

    # Crawlers go in in list order. Each level lists them by ascending number (hop.levels), so the Hash's
    # insertion order is the key order the spec iterates `crawlers.values` in; `patrol` replaces in place
    # and `remove` keeps the rest in order.
    def self.start(level)
      l = Levels.level(level)
      crawlers = l.crawlers.to_h { [it.no, it] }.freeze
      world = new(stage: l, ticks: 0, player: Model::Player.new(at: l.start), taken: NO_GEMS,
                  crawlers: crawlers, lives: 3, mode: :Playing)
      Outcome.new(world, ["0 level #{l.name}"])
    end

    def summary
      "end #{ticks} #{MODE_TEXT.fetch(mode)} gems #{taken.size}/#{stage.gems.size} lives #{lives} " \
        "at #{Geom.vec_text(player.at)}"
    end

    def view = View.new(stage, ticks, player, taken, crawlers.values, lives, mode)
  end
end

require_relative "world/step"
