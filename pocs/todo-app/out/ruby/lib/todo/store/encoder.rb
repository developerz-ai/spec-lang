# frozen_string_literal: true

# Projected by hand from the store format as written: state tasks and next_no in canonical bytes (docs/contract.md section 3, "Bytes written"). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "decoder"

module Todo
  module Store
    # The world to its one canonical line. Two backends given the same world write the same bytes.
    module Encoder
      # The world is past what the format holds: next_no above 2^53.
      class Full < StandardError; end

      NEEDS_ESCAPE = /["\\]/
      # A task's line beyond its title and tags, with room to spare: sizes the buffer once.
      TASK_OVERHEAD = 96

      def self.encode(world)
        raise Full if world.next_no > Decoder::MAX_NEXT_NO

        tasks = world.tasks.values.sort_by!(&:no)
        out = String.new(capacity: 64 + tasks.sum { |task| TASK_OVERHEAD + task.title.bytesize }, encoding: Encoding::UTF_8)
        out << %({"version":#{Decoder::VERSION},"next_no":#{world.next_no},"tasks":[)
        tasks.each_with_index do |task, index|
          out << "," unless index.zero?
          task(out, task)
        end
        out << "]}\n"
      end

      def self.task(out, task)
        out << %({"no":#{task.no},"title":"#{title(task.title)}","status":"#{task.status.name}",)
        out << %("priority":"#{task.priority.name}","tags":[#{tags(task.tags)}]})
      end

      # A Tag is a-z, 0-9 and "-": nothing in it is ever escaped.
      def self.tags(tags)
        tags.empty? ? "" : %("#{tags.sort.join('","')}")
      end

      # A Title holds no byte below 32, so `"` and `\` are all JSON needs escaped; the rest is raw UTF-8.
      def self.title(title)
        title.match?(NEEDS_ESCAPE) ? title.gsub(NEEDS_ESCAPE) { |char| "\\#{char}" } : title
      end

      private_class_method :task, :tags, :title
    end
  end
end
