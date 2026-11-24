# frozen_string_literal: true

# The test runtime of the Ruby printer of spec-lang build: tests, cases and properties.
# The same in every generated project; change the printer, not this file.

module Spec
  module Testing
    # How one check ended: "pass", "fail" or "blocked", and why when it did not pass.
    Outcome = Data.define(:status, :code, :message) do
      def self.pass = new("pass", nil, nil)

      # A check is blocked, not failed, when it reached a hole or a foreign body.
      def self.of(error, blocking: %w[B-FOREIGN])
        code = Testing.code_of(error)
        new(blocking.include?(code) ? "blocked" : "fail", code, error.message)
      end
    end

    def self.code_of(error)
      case error
      when Stop, Blocked then error.code
      when Failure, CheckFailed then "F-TEST"
      else "E-RUNTIME"
      end
    end

    # A test that did not reach its end: `status` is "fail" or "blocked".
    class Ended < StandardError
      attr_reader :status, :code

      def initialize(status, code, message)
        @status = status
        @code = code
        super(message)
      end

      def outcome = Outcome.new(code == "B-FOREIGN" ? "blocked" : status, code, message)
    end

    # One `test`: its parameters (a property's), its `@given` items, its steps.
    Test = Data.define(:name, :body, :params, :given)

    # A `@given` item: a state set to a value, or the end world of another test.
    SetState = Data.define(:state, :value)
    After = Data.define(:test)

    # One check: its name as `spec-lang test` prints it, and how to run it.
    Check = Data.define(:name, :kind, :run)

    # Every test and check of a program, in the order the printed files declare them.
    class Suite
      attr_reader :program, :checks, :config

      def initialize(program)
        @program = program
        @tests = {}
        @checks = []
        @ends = {}
        @running = []
        @config = Config.default
      end

      def configure(config) = @config = config

      def set(state, &value) = SetState.new(state:, value:)

      def after(test) = After.new(test:)

      def test(name, body, params: [], given: [])
        @tests[name] = Test.new(name:, body:, params:, given:)
      end

      # A test without parameters is one check: its `@given` world, then its steps.
      def scenario(name) = add(name, :test) { outcome_of { end_of(name) } }

      # A test with parameters: `runs` runs with generated arguments; the first that does not pass decides.
      def property(name) = add(name, :property) { property_outcome(@tests.fetch(name)) }

      # A `@cases` row on a fn: one call on a fresh machine, compared with a value or a refusal's name.
      def case(name, call, gives: nil, fails: nil)
        add(name, :case) { case_outcome(call, gives, fails) }
      end

      # A `@cases` row on a test: its `@given` world, then the test with the row's values.
      def case_of(name, test, args)
        add(name, :case) { outcome_of(blocking: %w[B-FOREIGN F-BLOCKED]) { row_of(@tests.fetch(test), args) } }
      end

      def law(name) = add(name, :law) { LawRun.new(self, @program.laws.find { |law| law.name == name }).outcome }

      def machine = Machine.new(@program)

      # Runs a test on a fresh machine at its `@given` world; gives the end world. Raises Ended.
      def run_test(test, args)
        Spec.on(given(test)) do
          guarded { test.body.call(*args) }
          Spec.world
        end
      end

      private

      def add(name, kind, &run) = @checks << Check.new(name:, kind:, run:)

      def outcome_of(blocking: %w[B-FOREIGN])
        yield
        Outcome.pass
      rescue Ended => e
        blocking.include?(e.code) ? Outcome.new("blocked", e.code, e.message) : e.outcome
      rescue StandardError, SystemStackError => e
        Outcome.of(e, blocking:)
      end

      # The steps of a test: a hole or a foreign body blocks it; a refusal nothing expected fails it.
      def guarded
        yield
      rescue Blocked => e
        raise Ended.new("blocked", e.code, e.message)
      rescue Failure => e
        raise Ended.new("fail", "F-TEST", "this step was refused: #{Spec.show(e)}")
      rescue StandardError, SystemStackError => e
        raise Ended.new("fail", Testing.code_of(e), e.message)
      end

      # The end world of a test without parameters, run once and remembered (also when it did not pass).
      def end_of(name)
        @ends[name] ||= begin
          [:world, run_test(@tests.fetch(name), [])]
        rescue Ended => e
          [:ended, e]
        end
        kind, result = @ends[name]
        raise result if kind == :ended

        result
      end

      def given(test)
        machine = fresh
        test.given.each do |item|
          case item
          when SetState then set_state(machine, item)
          else machine.world = parent_world(test, item.test)
          end
        end
        machine
      end

      def fresh
        machine
      rescue StandardError => e
        raise Ended.new("fail", Testing.code_of(e), e.message)
      end

      def set_state(machine, item)
        state = @program.states.find { |s| s.name == item.state }
        value = Spec.on(machine) { item.value.call }
        state.type.check(value, "this `@given` value") if state.type.strict?
        machine.world = machine.world.with(state.member => value)
      rescue StandardError => e
        raise Ended.new("fail", Testing.code_of(e), e.message)
      end

      def parent_world(test, parent)
        if @running.include?(parent) || parent == test.name
          raise Ended.new("fail", "E-TYPE-MISMATCH", "`@given` tests form a cycle")
        end
        unless @tests.fetch(parent).params.empty?
          raise Ended.new("fail", "E-TYPE-MISMATCH", "`@given` names a test with parameters; it has no one end world")
        end

        @running.push(test.name)
        begin
          end_of(parent)
        rescue Ended => e
          foreign = e.status == "blocked" && e.code == "B-FOREIGN"
          raise Ended.new("blocked", foreign ? "B-FOREIGN" : "F-BLOCKED", "blocked by #{parent}")
        ensure
          @running.pop
        end
      end

      def property_outcome(test)
        @config.runs.times do |run|
          outcome = outcome_of { run_test(test, arguments(test, run)) }
          return outcome unless outcome.status == "pass"
        end
        Outcome.pass
      end

      def arguments(test, run)
        generator = Generator.new(Rng.new(Rng.seed_for(@config.seed, test.name, run)))
        Spec.on(fresh) { test.params.map { |param| generator.value(param.type) } }
      end

      def row_of(test, args)
        values = Spec.on(given(test)) { guarded_row { args.call } }
        run_test(test, values)
      end

      def guarded_row
        yield
      rescue StandardError => e
        raise Ended.new("fail", Testing.code_of(e), e.message)
      end

      def case_outcome(call, gives, fails)
        outcome_of(blocking: %w[B-FOREIGN F-BLOCKED]) do
          Spec.on(machine) do
            got = Testing.outcome(call)
            same = fails ? got.is_a?(Failure) && got.name == fails : !got.is_a?(Failure) && got == gives.call
            raise CheckFailed, "the case gave #{Spec.show(got)}" unless same
          end
        end
      end
    end

    # The suite the printed check files add to.
    def self.suite(program = nil)
      @suite ||= Suite.new(program)
    end
  end
end
