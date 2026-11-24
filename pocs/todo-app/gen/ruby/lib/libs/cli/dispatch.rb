# frozen_string_literal: true

# The Ruby runtime of the `cli` library (libs/cli/real/ruby): one command against the store.
# The same in every app that uses `cli`; change the library, not this file.

require_relative "store/file_store"

module Libs
  module Cli
    # The real `out` handler: lines are kept until the rule commits; a refused rule takes its lines back.
    class Output
      attr_reader :lines

      def initialize = @lines = []

      def perform(operation, args)
        raise ArgumentError, "out.#{operation} is for tests" unless operation == :line

        @lines << args.fetch(0)
        nil
      end

      def mark = @lines.size

      def rewind(mark) = @lines.slice!(mark..)
    end

    # Runs one parsed command: load every state, one lock around the rule, an atomic replace after it, and
    # only then its lines.
    module Dispatch
      def self.call(program, app, invocation, env:, out:, err:)
        output = Output.new
        store(program, app, env).transact(writes?(program, invocation.command.rule)) do |world|
          machine = Spec::Machine.new(program, world)
          machine.serve("cli.out", output)
          Spec.on(machine) do
            invocation.command.rule.call(invocation.args)
            machine.world
          end
        end
        # After commit, and after the lock is released: a slow reader of our output holds nobody up.
        out.write(output.lines.map { |line| "#{line}\n" }.join)
        OK
      rescue Spec::Failure => e
        refuse(err, e.name, REFUSED)
      rescue Store::Contended
        refuse(err, "Contended", REFUSED)
      rescue Store::Failure => e
        refuse(err, e.message, STORE)
      end

      def self.store(program, app, env)
        at = app.note("store")["at"]
        raise ArgumentError, "@store(#{at.name}(...)): this runtime keeps a store in a file" unless at.name == "file"

        configured = env["#{app.note("bin")["name"].upcase}_STORE"]
        path = configured.nil? || configured.empty? ? at.args.fetch("path") : configured
        Store::FileStore.new(path, Store::Schema.new(program, app))
      end

      # Whether the rule states that it writes a state (`@uses(tasks.write)`). A rule that states no effect
      # at all is taken to write.
      def self.writes?(program, rule)
        return true if rule.uses.empty?

        states = program.states.map(&:name)
        rule.uses.any? do |use|
          use.end_with?(".write") && states.any? { |name| ".#{name}".end_with?(".#{use.delete_suffix(".write")}") }
        end
      end

      def self.refuse(err, message, code)
        err.write("error: #{message}\n")
        code
      end
    end
  end
end
