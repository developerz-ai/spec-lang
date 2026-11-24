# frozen_string_literal: true

# Runtime shim of the ruby backend of spec-lang build: test board, generators, traces. The same in every generated project; change the backend, not this file.

require "runtime/values"

module Runtime
  # What the generated tests stand on: a board to call rules on, seeded generators, traces.
  module Testing
    # Every property and law: this many runs from this seed, so each run of the suite is the same.
    SEED = Integer(ENV.fetch("SPEC_SEED", "20261124"))
    RUNS = 200
    # Events in the trace a law is checked on, and the most events run before it to reach its start world.
    TRACE_LENGTH = 40
    PREFIX_MAX = 20

    # A test's world: rules are called directly, `out.line` collects into `out_text`.
    # Not a mock: the rules are the real ones.
    class Board
      attr_accessor :world
      attr_reader :out_text

      def initialize(world)
        @world = world
        @out_text = +""
      end

      # "when": commits the new world, sends the lines, gives the reply.
      def call
        outcome = yield @world
        @world = outcome.world
        outcome.lines.each { |line| @out_text << line << "\n" }
        outcome.reply
      end

      # `-> fail E`: the name of the error the rule refused with.
      def refused
        yield @world
        raise "expected a refusal"
      rescue Refusal => e
        e.error_name
      end

      # A test that starts from another test's end state starts with nothing printed.
      def forget_output = @out_text = +""
    end

    # Values generated from types, from one seed.
    class Gen
      WORD = [*"a".."z", *"0".."9", "-"].freeze
      ANY = [*WORD, " ", "A", "Z", "_", "\t", "\n", "é", "☕", "\u{1F600}", "　", "\u0085", "﻿", '"', "\\", "#"].freeze
      FIXED = ["", " ", "a b", "  padded  ", "x" * 200, "x" * 201].freeze

      attr_reader :seed

      def initialize(seed)
        @seed = seed
        @random = Random.new(seed)
      end

      # `Int`: mostly small, so numbers meet the ones a program holds; now and then past every machine word.
      def int
        case @random.rand(10)
        when 0 then @random.rand(-(2**70)..(2**70))
        when 1 then @random.rand(-16..-1)
        else @random.rand(0..16)
        end
      end

      # `Text`: half plain words, so tight claims are met often; half anything.
      def text
        case @random.rand(8)
        when 0 then FIXED.sample(random: @random)
        when 1..4 then Array.new(@random.rand(1..8)) { WORD.sample(random: @random) }.join.freeze
        else Array.new(@random.rand(0..12)) { ANY.sample(random: @random) }.join.freeze
        end
      end

      def bool = @random.rand(2).zero?

      def pick(all) = all.sample(random: @random)

      # Values of a claimed type: values of its base that meet the claim. A claim too tight to meet is an error.
      def claimed(type)
        1000.times do
          value = yield
          return value if type.valid?(value)
        end
        raise "no #{type::NAME} could be generated"
      end

      def set = Array.new(@random.rand(0..3)) { yield }.to_set.freeze

      def option = @random.rand(2).zero? ? nil : yield

      def list(max) = Array.new(@random.rand(0..max)) { yield }
    end

    # One element of `t.steps`. `reply` is the rule's reply, or the Refusal it failed with.
    Step = Data.define(:event, :before, :after, :reply)
    Trace = Data.define(:states, :steps)

    def self.runs
      RUNS.times { |index| yield Gen.new(SEED + index) }
    end

    # The world after `events`; a refused event leaves it as it was. Every such world is one the rules can reach.
    def self.reach(start, events)
      events.reduce(start) do |world, event|
        yield(world, event).world
      rescue Refusal
        world
      end
    end

    def self.run_trace(start, events)
      states = [start]
      steps = events.map do |event|
        before = states.last
        step = begin
          outcome = yield(before, event)
          Step.new(event:, before:, after: outcome.world, reply: outcome.reply)
        rescue Refusal => e
          Step.new(event:, before:, after: before, reply: e)
        end
        states << step.after
        step
      end
      Trace.new(states:, steps:)
    end
  end
end
