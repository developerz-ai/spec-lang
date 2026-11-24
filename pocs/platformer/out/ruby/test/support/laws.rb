# frozen_string_literal: true

# Projected by hand from Play and every law of hop.laws (src/hop/laws.slg). Stand-in for spec-lang build output; change the spec, not this file.

module Skyhop
  module TestSupport
    # Each law as a check on one state (Trace[Play]) or one step (Trace[world.step]); nil when it holds.
    module Laws
      P = Physics
      Step = Data.define(:before, :after, :reply)

      STATE = {
        never_inside_a_solid: lambda { |w|
          w.stage.solids.none? { |s| Geom.overlaps(P.body(w.player.at), s.box) }
        },
        speed_is_bounded: lambda { |w|
          v = w.player.vel
          v.x.abs <= P::RUN_SPEED && v.z.abs <= P::RUN_SPEED && (-P::MAX_FALL..P::JUMP_SPEED).cover?(v.y)
        },
        lost_means_no_lives: ->(w) { w.lives.zero? == (w.mode == :Lost) },
        taken_are_real_gems: ->(w) { w.taken.all? { |n| w.stage.gems.any? { it.no == n } } }
      }.freeze

      STEP = {
        taken_only_grows: ->(s) { s.before.taken.subset?(s.after.taken) },
        lives_never_rise: ->(s) { s.after.lives <= s.before.lives },
        over_is_final: ->(s) { s.before.mode == :Playing || (s.reply == :Over && s.after == s.before) },
        one_tick_per_step: ->(s) { s.reply == :Over || s.after.ticks == s.before.ticks + 1 }
      }.freeze

      module_function

      def broken_by_state(world) = STATE.reject { |_, law| law.call(world) }.keys

      def broken_by_step(step) = STEP.reject { |_, law| law.call(step) }.keys

      # Calls step, recording a refusal as the reply :Over and the world unchanged.
      def step(world, pad)
        outcome = world.step(pad)
        Step.new(world, outcome.world, :Ok)
      rescue World::Over
        Step.new(world, world, :Over)
      end
    end
  end
end
