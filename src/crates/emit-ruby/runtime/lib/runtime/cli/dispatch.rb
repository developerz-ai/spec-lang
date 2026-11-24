require_relative "command"
require_relative "../values"
require_relative "../store/file_store"

module Runtime
  module CLI
    # Runs one parsed command against the store and reports it.
    module Dispatch
      def self.call(app, invocation, path:, out:, err:)
        outcome = run(app, invocation, path)
        # After commit, and after the lock is released: a slow reader of our output holds nobody up.
        out.write(outcome.lines.map { |line| "#{line}\n" }.join)
        ExitCode::OK
      rescue Refusal => e
        refuse(err, e.error_name, ExitCode::REFUSED)
      rescue Store::Contended
        refuse(err, "Contended", ExitCode::REFUSED)
      rescue Store::Failure => e
        refuse(err, e.message, ExitCode::STORE)
      end

      # Only the rule's own file is loaded, and of the store only what the rule's effects need.
      def self.run(app, invocation, path)
        command = invocation.command
        require command.load
        codec, initial = app.store.call
        Store::FileStore.new(path, codec:, initial:).transact(command.access) do |world|
          command.rule.call(world, invocation.args)
        end
      end

      def self.refuse(err, message, code)
        err.write("error: #{message}\n")
        code
      end
    end
  end
end
