# frozen_string_literal: true

# The Ruby runtime of the `cli` library (libs/cli/real/ruby): the store file, from the states' declared types.
# The same in every app that uses `cli`; change the library, not this file.

require_relative "json_reader"
require_relative "codec"

module Libs
  module Cli
    module Store
      # The whole store: one line of JSON, `{"version":1,...}`, then every state of the modules that hold the
      # app's rules: those that are not collections first, then the collections, each in declared order.
      # A store is readable when it is that shape, every value is of its state's type, and every law of the
      # program that speaks of single states (`t.states.all(w => p)`) holds on it: a file the rules could not
      # have written is refused.
      class Schema
        VERSION = 1

        def initialize(program, app)
          @program = program
          modules = app.rules.map { |name| name.rpartition(".").first }.uniq
          states = program.states.select { |state| modules.include?(state.name.rpartition(".").first) }
          scalars, collections = states.partition { |state| !%i[map set list].include?(Codec.base(state.type).kind) }
          @fields = (scalars + collections).map { |state| [key(state, states), state] }
          @laws = program.laws.select { |law| law.params.size == 1 }
        end

        def initial = @program.initial_world

        # Store text (tagged UTF-8, not yet checked) to the world.
        def decode(text)
          json = JsonReader.parse(text)
          Codec.malformed("version") unless json.is_a?(Hash) && json["version"] == VERSION
          Codec.malformed("keys") unless json.size == @fields.size + 1
          values = @fields.to_h do |key, state|
            [state.member, Codec.decode(state.type, json.fetch(key) { Codec.malformed("no #{key}") })]
          end
          lawful(initial.with(**values))
        rescue JsonReader::Invalid, Spec::Stop, Spec::Failure => e
          raise Malformed, e.message
        end

        # The world to its one canonical line.
        def encode(world)
          out = String.new(capacity: 256, encoding: Encoding::UTF_8)
          out << '{"version":' << VERSION.to_s
          @fields.each do |key, state|
            out << ',"' << key << '":'
            Codec.encode(state.type, world.public_send(state.member), out)
          end
          out << "}\n"
        end

        private

        # A state's key is its name; two states of one name in different modules take their whole path.
        def key(state, all)
          short = state.name.rpartition(".").last
          all.count { |other| other.name.rpartition(".").last == short } == 1 ? short : state.name
        end

        def lawful(world)
          trace = Spec::Trace.new(steps: [], states: [world])
          held = Spec.on(Spec::Machine.new(@program, world)) { @laws.all? { |law| law.body.call(trace) == true } }
          Codec.malformed("law") unless held
          world
        end
      end
    end
  end
end
