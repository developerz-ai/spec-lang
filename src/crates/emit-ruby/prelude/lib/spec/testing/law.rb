# frozen_string_literal: true

# The test runtime of the Ruby printer of spec-lang build: laws over random traces.
# The same in every generated project; change the printer, not this file.

module Spec
  module Testing
    # A law holds on `traces` random traces of `steps` calls. Trace `i` comes from the seed of (law, i): an odd
    # `i` starts at a drawn world on which every law holds, an even one at the initial world after up to `prefix`
    # random calls. The engine draws the same traces from the same seed.
    class LawRun
      WORLD_TRIES = 200

      def initialize(suite, law)
        @suite = suite
        @program = suite.program
        @config = suite.config
        @law = law
      end

      def outcome
        @config.traces.times do |index|
          outcome = one(index)
          return outcome unless outcome.status == "pass"
        end
        Outcome.pass
      end

      private

      def one(index)
        @generator = Generator.new(Rng.new(Rng.seed_for(@config.seed, @law.name, index)))
        @rng = @generator.instance_variable_get(:@rng)
        start = index.odd? ? lawful_world : nil
        steps, worlds = trace(start)
        others = other_params(@law, worlds)
        case verdict(@law, steps, worlds, others)
        when true then Outcome.pass
        when false then Outcome.new("fail", "F-LAW", "law #{@law.name} is false on trace #{index}")
        end
      rescue StandardError, SystemStackError => e
        Outcome.of(e)
      end

      # Every state drawn from its type, kept when every law of the program holds on that world alone.
      def lawful_world
        Spec.on(@suite.machine) do
          WORLD_TRIES.times do
            world = @program.world_class.new(*@program.states.map { |state| @generator.value(state.type) })
            return world if lawful?(world)
          end
        end
        nil
      end

      def lawful?(world)
        @program.laws.all? do |law|
          others = other_params(law, [world])
          begin
            verdict(law, [], [world], others) == true
          rescue StandardError, SystemStackError
            false
          end
        end
      end

      # The law's other parameters, drawn with pools of every world of the trace.
      def other_params(law, worlds)
        pools = Pools.from_worlds(@program, worlds)
        Spec.on(@suite.machine) do
          law.params.each_with_index.reject { |_, i| i == law.trace }.map { |param, _| @generator.value(param.type, pools) }
        end
      end

      def verdict(law, steps, worlds, others)
        args = others.dup.insert(law.trace, Trace.new(steps:, states: worlds))
        held = Spec.on(@suite.machine) { law.body.call(*args) }
        Spec.stop!("E-TYPE-MISMATCH", "a law is a Bool") unless [true, false].include?(held)
        held
      rescue Failure => e
        raise Stop.new("F-LAW", "the law failed with #{e.name} instead of giving a Bool")
      end

      def trace(start)
        machine = @suite.machine
        Spec.on(machine) do
          if start
            machine.world = start
          else
            @rng.below(@config.prefix + 1).times { call(machine, random_event(machine, @law.start)) }
          end
          steps = []
          worlds = [machine.world]
          @config.steps.times do
            event = random_event(machine, @law.traced)
            before = machine.world
            reply = call(machine, event)
            steps << Step.new(event:, reply:, before:, after: machine.world)
            worlds << machine.world
          end
          [steps, worlds]
        end
      end

      def random_event(machine, rules)
        Spec.stop!("E-GEN", "a trace with no rules") if rules.empty?
        rule = @program.rules.fetch(@rng.pick(rules))
        pools = Pools.from_worlds(@program, [machine.world])
        Event.new(rule:, args: rule.params.map { |param| @generator.value(param.type, pools) })
      end

      # One call from outside: its value, or the refusal it gave.
      def call(_machine, event)
        Ok.new(event.rule.call(event.args))
      rescue Failure => e
        Err.new(e)
      end
    end
  end
end
