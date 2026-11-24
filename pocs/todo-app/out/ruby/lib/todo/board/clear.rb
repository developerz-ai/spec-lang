# frozen_string_literal: true

# Projected by hand from rule clear (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "world"
require_relative "outcome"

module Todo
  module Board
    # Finished work leaves the list in one step.
    def self.clear(world)
      gone = world.tasks.each_value.select { |task| task.status == Model::Status::DONE }
      after = gone.empty? ? world : world.without(gone.map(&:no))
      Outcome.new(world: after, lines: ["cleared #{gone.size}"], reply: gone.size)
    end
  end
end
