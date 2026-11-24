# frozen_string_literal: true

# Projected by hand from the store format as read: state tasks and next_no with every claim checked (docs/contract.md section 3, "Accepted on read"). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "json_reader"
require_relative "../board/counts"
require_relative "../model/title"
require_relative "../model/tag"
require_relative "../model/status"
require_relative "../model/priority"

module Todo
  module Store
    # Store text to the world, or to the part of it a rule reads. Every byte is checked either way.
    # The text is tagged UTF-8; whether it is valid UTF-8 is checked here.
    module Decoder
      class Malformed < StandardError; end

      VERSION = 1
      MAX_NEXT_NO = 2**53
      TOP_KEYS = %w[version next_no tasks].freeze
      TASK_KEYS = %w[no title status priority tags].freeze
      STATUSES = Model::Status::ALL.to_h { |variant| [variant.name, variant] }.freeze
      PRIORITIES = Model::Priority::ALL.to_h { |variant| [variant.name, variant] }.freeze

      # The Task constructor checks the claims on title and tags, so they are checked once.
      # Tasks and the world are loaded here, so a rule that reads only counts never loads them.
      def self.world(text)
        require_relative "../board/world"
        tasks = {}
        next_no = each_task(text) do |no, title, status, priority, tags|
          tasks[no] = Model::Task.new(no:, title:, status:, priority:, tags: tags.to_set)
        end
        Board::World.new(tasks: tasks.freeze, next_no:)
      rescue Model::Invalid => e
        raise Malformed, e.message
      end

      # What `stats` reads. The same checks as `world`, and no task, set or map is built.
      def self.counts(text)
        open = 0
        size = 0
        each_task(text) do |_no, title, status, _priority, tags|
          malformed("title") unless Model.title?(title)
          malformed("tags") unless tags.all? { |tag| Model.tag?(tag) }
          size += 1
          open += 1 if status == Model::Status::OPEN
        end
        Board::Counts.new(open:, size:)
      end

      # Yields each task's fields, checked for everything but the claims on title and tags; returns next_no.
      def self.each_task(text)
        top = record(JsonReader.parse(text), TOP_KEYS)
        malformed("version") unless top["version"] == VERSION
        next_no = integer(top["next_no"], 1..MAX_NEXT_NO)
        seen = {}
        array(top["tasks"]).each do |json|
          fields = task(json, next_no)
          malformed("no twice") if seen.key?(fields.first)
          yield seen[fields.first] = fields
        end
        next_no
      rescue JsonReader::Invalid => e
        raise Malformed, e.message
      end

      def self.task(json, next_no)
        task = record(json, TASK_KEYS)
        [integer(task["no"], 1...next_no), task["title"], variant(STATUSES, task["status"]),
         variant(PRIORITIES, task["priority"]), tags(task["tags"])]
      end

      def self.record(json, keys)
        malformed("keys") unless json.is_a?(Hash) && json.size == keys.size && keys.all? { |key| json.key?(key) }
        json
      end

      def self.integer(json, range)
        malformed("integer") unless json.is_a?(Integer) && range.cover?(json)
        json
      end

      def self.array(json)
        malformed("array") unless json.is_a?(Array)
        json
      end

      def self.variant(variants, json)
        variants.fetch(json) { malformed("variant") }
      end

      def self.tags(json)
        malformed("tag twice") unless array(json).uniq.size == json.size
        json
      end

      def self.malformed(what) = raise(Malformed, what)

      private_class_method :each_task, :task, :record, :integer, :array, :variant, :tags, :malformed
    end
  end
end
