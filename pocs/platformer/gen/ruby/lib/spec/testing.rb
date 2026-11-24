# frozen_string_literal: true

# The test runtime of the Ruby printer of spec-lang build: the program's own checks, run as `spec-lang test` runs them.
# The same in every generated project; change the printer, not this file.

require_relative "../spec"
require_relative "testing/rng"
require_relative "testing/generate"
require_relative "testing/suite"
require_relative "testing/law"
require_relative "testing/runner"

module Spec
  # Cases, scenarios with their `@given` worlds, properties and laws, with the engine's seeds and generator, so a
  # property draws the arguments and a law the traces `spec-lang test` draws.
  module Testing
    # Counts and the base seed, as `spec-lang test` takes them.
    Config = Struct.new(:seed, :runs, :traces, :steps, :prefix, :jobs) do
      def self.default = new(0, 100, 200, 40, 10, nil)
    end

    # `subject -> value;`: the step's value must equal the expected one, evaluated after it.
    def self.gives(subject)
      got = outcome(subject)
      wanted = yield
      return if !got.is_a?(Failure) && got == wanted

      raise CheckFailed, "the step gave #{Spec.show(got)}, expected #{Spec.show(wanted)}"
    end

    # `subject -> fail Name;`: the step must be refused by that name, and with that data when data is given.
    def self.fails(name, subject)
      got = outcome(subject)
      data = block_given? ? yield : []
      return if got.is_a?(Failure) && got.name == name && (data.empty? || got.data == data)

      raise CheckFailed, "the step gave #{Spec.show(got)}, expected #{Spec.show(Failure.new(name, data))}"
    end

    def self.outcome(subject)
      subject.call
    rescue Failure => e
      e
    end
  end
end
