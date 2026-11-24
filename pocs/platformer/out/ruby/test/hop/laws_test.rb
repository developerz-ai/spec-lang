# frozen_string_literal: true

# Projected by hand from every law of hop.laws, run over seeded traces of start then step (src/hop/laws.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "../test_helper"
require_relative "../support/laws"

module Skyhop
  module TestSupport
    # 200 traces of 200 ticks: start a random level, then random pads held for random spans, every law after
    # every step. Two pads in three hold right, so traces cross gaps, take gems and fall, not only stand.
    # No trace from start ends a game within 200 ticks (a win takes 316, a fall about 125), so 50 more traces
    # start from the named start worlds of hop.world, two of which are a tick from the end: the card lets a
    # trace start from a @given. Seed from SKYHOP_SEED (default 20261124); trace i uses seed + i, and a failure names it.
    class LawsTest < Minitest::Test
      TRACES = 200
      TICKS = 200
      GIVEN_TRACES = 50
      SEED = Integer(ENV.fetch("SKYHOP_SEED", "20261124"))
      PADS = (0...32).map do |bits|
        Pad.new(left: bits[0] == 1, right: bits[1] == 1, forward: bits[2] == 1, back: bits[3] == 1, jump: bits[4] == 1)
      end.freeze

      def test_every_law_holds_after_every_step
        seen = Hash.new(0)
        TRACES.times { |i| trace(SEED + i, seen) { World.start(it.rand(1..2)).world } }
        names = StartWorlds::WORLDS.keys
        GIVEN_TRACES.times { |i| trace(SEED + TRACES + i, seen) { StartWorlds.world(names[i % names.size]) } }

        # The traces must reach the parts of the game the laws speak of, or they would hold by not being tested.
        warn "laws: #{seen.sort.map { |k, v| "#{k} #{v}" }.join(", ")}" if ENV["SKYHOP_TRACE_STATS"]

        %i[jump land gem fall Over].each { |event| assert_operator seen[event], :>, 0, "no trace reached #{event}" }
      end

      def trace(seed, seen)
        rng = Random.new(seed)
        world = yield rng
        check(seed, 0, Laws.broken_by_state(world))
        pad = PADS.first
        TICKS.times do |tick|
          pad = next_pad(rng) if rng.rand(6).zero?
          step = Laws.step(world, pad)
          record(step, seen)
          check(seed, tick + 1, Laws.broken_by_step(step) + Laws.broken_by_state(step.after))
          world = step.after
        end
      end

      def next_pad(rng)
        pad = PADS.sample(random: rng)
        rng.rand(3).zero? ? pad : pad.with(right: true, left: false)
      end

      def record(step, seen)
        seen[:Over] += 1 if step.reply == :Over
        return if step.reply == :Over

        seen[:jump] += 1 if step.after.player.vel.y == Physics::JUMP_SPEED - Physics::GRAVITY
        seen[:land] += 1 if step.after.player.grounded && !step.before.player.grounded
        seen[:gem] += 1 if step.after.taken.size > step.before.taken.size
        seen[:fall] += 1 if step.after.lives < step.before.lives
      end

      def check(seed, tick, broken)
        assert_empty broken, "seed #{seed}, tick #{tick}: #{broken.join(", ")}"
      end
    end
  end
end
