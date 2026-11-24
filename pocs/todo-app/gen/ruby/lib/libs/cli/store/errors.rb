# frozen_string_literal: true

# The Ruby runtime of the `cli` library (libs/cli/real/ruby): what the store can refuse.
# The same in every app that uses `cli`; change the library, not this file.

module Libs
  module Cli
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
end
