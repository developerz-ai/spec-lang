# frozen_string_literal: true

# Projected by hand from how a spec test calls rules on the one running world (docs/idea/01-language-card.md, how it runs). Stand-in for spec-lang build output; change the spec, not this file.

module Skyhop
  module TestSupport
    # The world a spec test runs against: a rule call commits its outcome; a refusal leaves the world as it was.
    # `out` is `out.text`: every line the committed rules sent, each followed by a newline.
    class Session
      attr_reader :world, :out

      def initialize(world)
        @world = world
        @out = +""
      end

      def start(level) = commit(World.start(level))

      def step(pad) = commit(@world.step(pad))

      def player = @world.player

      private

      def commit(outcome)
        @world = outcome.world
        outcome.lines.each { @out << it << "\n" }
        outcome.lines
      end
    end
  end
end
