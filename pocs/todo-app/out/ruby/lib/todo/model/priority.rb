# frozen_string_literal: true

# Projected by hand from type Priority (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

module Todo
  module Model
    # A sum with no data: each variant is the symbol of its name.
    module Priority
      LOW = :Low
      NORMAL = :Normal
      HIGH = :High
      ALL = [LOW, NORMAL, HIGH].freeze
    end
  end
end
