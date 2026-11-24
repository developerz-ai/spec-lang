# frozen_string_literal: true

# Projected by hand from the errors NoTask, AlreadyDone, NotDone named by @guard (src/todo/board.slg). Stand-in for spec-lang build output; change the spec, not this file.

module Todo
  module Board
    # A rule refused. Raised before any new world exists, so the world is unchanged.
    class Refusal < StandardError
      def self.error_name = name.split("::").last

      def initialize(message = self.class.error_name) = super

      def error_name = self.class.error_name
    end

    class NoTask < Refusal; end
    class AlreadyDone < Refusal; end
    class NotDone < Refusal; end
  end
end
