# frozen_string_literal: true

# Runtime shim of the ruby backend of spec-lang build: store errors. The same in every generated project; change the backend, not this file.

module Runtime
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
