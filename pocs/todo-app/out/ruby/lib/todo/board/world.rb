# frozen_string_literal: true

# Projected by hand from state tasks and state next_no (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "../model/task"
require_relative "counts"

module Todo
  module Board
    # Every state of the module as one immutable value. A rule returns a new one; nothing is changed in place.
    World = Data.define(:tasks, :next_no) do
      def self.initial = new(tasks: {}, next_no: 1)

      def initialize(tasks:, next_no:)
        raise Model::Invalid, "next_no: not a TaskNo" unless Model.task_no?(next_no)
        raise Model::Invalid, "tasks: not a Map" unless tasks.is_a?(Hash)

        super(tasks: tasks.frozen? ? tasks : tasks.dup.freeze, next_no:)
      end

      # `tasks[no]` with a guard's `else`: the task, or the refusal.
      def task(no, otherwise)
        tasks.fetch(no) { raise otherwise }
      end

      def put(task) = with(tasks: tasks.merge(task.no => task).freeze)

      def without(nos) = with(tasks: tasks.except(*nos).freeze)

      def counts
        Counts.new(open: tasks.each_value.count { |task| task.status == Model::Status::OPEN }, size: tasks.size)
      end
    end
  end
end
