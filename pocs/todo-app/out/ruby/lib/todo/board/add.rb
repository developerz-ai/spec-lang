# frozen_string_literal: true

# Projected by hand from rule add (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "world"
require_relative "outcome"

module Todo
  module Board
    # A number is never reused, so an old note that says "task 3" never names a new task.
    def self.add(world, title:, priority: Model::Priority::NORMAL, tags: Set[])
      no = world.next_no
      task = Model::Task.new(no:, title:, priority:, tags:)
      after = World.new(tasks: world.tasks.merge(no => task).freeze, next_no: no + 1)
      Outcome.new(world: after, lines: ["added #{no}"], reply: no)
    end
  end
end
