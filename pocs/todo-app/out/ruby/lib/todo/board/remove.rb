# frozen_string_literal: true

# Projected by hand from rule remove (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "world"
require_relative "outcome"
require_relative "refusals"

module Todo
  module Board
    def self.remove(world, no:)
      world.task(no, NoTask)
      Outcome.new(world: world.without([no]), lines: ["removed #{no}"], reply: nil)
    end
  end
end
