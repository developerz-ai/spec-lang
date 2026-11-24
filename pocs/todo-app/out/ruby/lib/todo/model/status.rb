# frozen_string_literal: true

# Projected by hand from type Status (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

module Todo
  module Model
    # A sum with no data: each variant is the symbol of its name.
    module Status
      OPEN = :Open
      DONE = :Done
      ALL = [OPEN, DONE].freeze
    end
  end
end
