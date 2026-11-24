# frozen_string_literal: true

# Projected by hand from the tests, laws and @cases of the program (src/todo/**/*.slg). Stand-in for spec-lang build output; change the spec, not this file.

require "minitest/autorun"
require "tmpdir"
require "stringio"
require "todo"

module Todo
  module TestSupport
    ROOT = File.expand_path("..", __dir__)
    EXE = File.join(ROOT, "exe", "todo")
  end
end
