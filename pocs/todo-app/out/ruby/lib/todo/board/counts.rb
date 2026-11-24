# frozen_string_literal: true

# Projected by hand from the read set of rule stats: tasks.size and each status (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

module Todo
  module Board
    # All that `stats` reads of the world. The store can produce it without building a task.
    Counts = Data.define(:open, :size) do
      def counts = self
    end
  end
end
