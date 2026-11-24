# frozen_string_literal: true

# Projected by hand from rule stats (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "counts"
require_relative "outcome"

module Todo
  module Board
    # Reads only `counts`, so it takes a World or the Counts the store read in its place.
    def self.stats(world)
      counts = world.counts
      Outcome.new(world:, lines: ["#{counts.open} open, #{counts.size - counts.open} done"], reply: nil)
    end
  end
end
