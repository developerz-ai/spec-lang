# frozen_string_literal: true

# Projected by hand from the cli vocabulary: load, one lock around the rule, replace, then out.line; fail E and store errors (docs/cli-vocabulary.md, docs/contract.md sections 2 and 3). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "exit_code"
require_relative "../board/refusals"
require_relative "../store/file_store"

module Todo
  module CLI
    # Runs one parsed command against the store and reports it.
    module Dispatch
      def self.call(invocation, path:, out:, err:)
        outcome = run(invocation, Store::FileStore.new(path))
        # After commit, and after the lock is released: a slow reader of our output holds nobody up.
        out.write(outcome.lines.map { |line| "#{line}\n" }.join)
        ExitCode::OK
      rescue Board::Refusal => e
        refuse(err, e.error_name, ExitCode::REFUSED)
      rescue Store::Contended
        refuse(err, "Contended", ExitCode::REFUSED)
      rescue Store::Failure => e
        refuse(err, e.message, ExitCode::STORE)
      end

      # Only the rule's own file is loaded, and of the store only what the rule's effects need.
      def self.run(invocation, store)
        command = invocation.command
        require_relative "../board/#{command.rule}"
        store.transact(command.access, reads: command.reads) do |world|
          Board.public_send(command.rule, world, **invocation.args)
        end
      end

      def self.refuse(err, message, code)
        err.write("error: #{message}\n")
        code
      end
    end
  end
end
