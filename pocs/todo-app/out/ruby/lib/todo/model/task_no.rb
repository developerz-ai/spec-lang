# frozen_string_literal: true

# Projected by hand from type TaskNo (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

module Todo
  module Model
    # Int is exact and unbounded, and so is Integer.
    def self.task_no?(value)
      value.is_a?(Integer) && value >= 1
    end
  end
end
