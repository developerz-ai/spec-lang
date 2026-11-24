# frozen_string_literal: true

# Projected by hand from fn has_tag and rule list (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "world"
require_relative "outcome"
require_relative "../render"

module Todo
  module Board
    # `tag` is an Option[Tag]: nil is none.
    def self.has_tag?(task, tag) = tag.nil? || task.tags.include?(tag)

    # Open work first; finished work only when asked for.
    def self.list(world, all: false, tag: nil)
      shown = world.tasks.each_value
                   .select { |task| (all || task.status == Model::Status::OPEN) && has_tag?(task, tag) }
                   .sort_by { |task| Render.rank(task) }
      lines = shown.empty? ? ["nothing to do"] : shown.map { |task| Render.line(task) }
      Outcome.new(world:, lines:, reply: nil)
    end
  end
end
