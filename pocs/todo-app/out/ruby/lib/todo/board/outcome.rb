# frozen_string_literal: true

# Projected by hand from the shape of every rule: new state, out.line effects, reply (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

module Todo
  module Board
    # What a rule that did not refuse gives back. Lines are sent by the caller, after commit.
    Outcome = Data.define(:world, :lines, :reply)
  end
end
