# frozen_string_literal: true

# Projected by hand from the store errors of the cli vocabulary (docs/contract.md section 2, "Errors"). Stand-in for spec-lang build output; change the spec, not this file.

module Todo
  module Store
    # The store could not serve the rule. `path` is the path as configured, not resolved.
    class Failure < StandardError
      attr_reader :path

      def initialize(path)
        @path = path
        super("store #{path}: #{self.class::REASON}")
      end
    end

    class Unreadable < Failure
      REASON = "unreadable"
    end

    class Unwritable < Failure
      REASON = "unwritable"
    end

    class Full < Failure
      REASON = "full"
    end

    # The lock was not had within its bound: the rule could not be ordered.
    class Contended < StandardError; end
  end
end
