# frozen_string_literal: true

# Projected by hand from rule reopen (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "world"
require_relative "outcome"
require_relative "refusals"
require_relative "../render"

module Todo
  module Board
    def self.reopen(world, no:)
      task = world.task(no, NoTask)
      raise NotDone unless task.status == Model::Status::DONE

      moved = task.with(status: Model::Status::OPEN)
      Outcome.new(world: world.put(moved), lines: [Render.line(moved)], reply: nil)
    end
  end
end
