# frozen_string_literal: true

# Projected by hand from fn mark, badge, tag_text, urgency, rank, line (src/todo/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "model/status"
require_relative "model/priority"

module Todo
  # How a task reads on one line. Pure.
  module Render
    MARKS = { Model::Status::OPEN => " ", Model::Status::DONE => "x" }.freeze
    BADGES = { Model::Priority::LOW => " !low", Model::Priority::NORMAL => "",
               Model::Priority::HIGH => " !high" }.freeze
    URGENCIES = { Model::Priority::HIGH => 0, Model::Priority::NORMAL => 1, Model::Priority::LOW => 2 }.freeze

    def self.mark(status) = MARKS.fetch(status)

    def self.badge(priority) = BADGES.fetch(priority)

    # `tags.sort(it)` is code-point order, which is byte order for the ASCII a Tag is made of.
    def self.tag_text(tags) = tags.sort.map { |tag| " ##{tag}" }.join

    def self.urgency(priority) = URGENCIES.fetch(priority)

    # Most urgent first, then oldest first.
    def self.rank(task) = [urgency(task.priority), task.no]

    def self.line(task)
      "#{task.no} [#{mark(task.status)}] #{task.title}#{badge(task.priority)}#{tag_text(task.tags)}"
    end
  end
end
