# frozen_string_literal: true

# Projected by hand from rule rename (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "world"
require_relative "outcome"
require_relative "refusals"
require_relative "../render"

module Todo
  module Board
    def self.rename(world, no:, title:)
      renamed = world.task(no, NoTask).with(title:)
      Outcome.new(world: world.put(renamed), lines: [Render.line(renamed)], reply: nil)
    end
  end
end
