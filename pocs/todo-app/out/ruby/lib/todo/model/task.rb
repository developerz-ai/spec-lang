# frozen_string_literal: true

# Projected by hand from type Task (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "invalid"
require_relative "title"
require_relative "tag"
require_relative "task_no"
require_relative "status"
require_relative "priority"

module Todo
  module Model
    # A record. Deeply frozen, compared by value; `with` re-checks every claim.
    Task = Data.define(:no, :title, :status, :priority, :tags) do
      def initialize(no:, title:, status: Status::OPEN, priority: Priority::NORMAL, tags: Set[])
        raise Invalid, "no: not a TaskNo" unless Model.task_no?(no)
        raise Invalid, "title: not a Title" unless Model.title?(title)
        raise Invalid, "status: not a Status" unless Status::ALL.include?(status)
        raise Invalid, "priority: not a Priority" unless Priority::ALL.include?(priority)
        raise Invalid, "tags: not a Set[Tag]" unless tags.is_a?(Set) && tags.all? { |tag| Model.tag?(tag) }

        super(no:, title: -title, status:, priority:, tags: tags.to_set(&:-@).freeze)
      end
    end
  end
end
