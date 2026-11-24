# frozen_string_literal: true

# Projected by hand from the tests, laws and @cases of the program (src/hop/**/*.slg). Stand-in for spec-lang build output; change the spec, not this file.

require "minitest/autorun"
require "stringio"
require_relative "../lib/skyhop/render"
require_relative "../lib/skyhop/script"
require_relative "../lib/skyhop/cli"
require_relative "support/session"
require_relative "support/start_worlds"

module Skyhop
  module TestSupport
    ROOT = File.expand_path("..", __dir__)
    POC = File.expand_path("../../..", __dir__)
    EXE = File.join(ROOT, "exe", "skyhop")
    Vec3 = Geom::Vec3
    Box = Geom::Box
    Pad = Model::Pad
    REST = Pad.new
    RIGHT = Pad.new(right: true)
    JUMP = Pad.new(jump: true)
  end
end
