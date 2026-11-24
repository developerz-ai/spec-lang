# frozen_string_literal: true

# The Ruby runtime of the `cli` library (libs/cli/real/ruby): one run of the command line.
# The same in every app that uses `cli`; change the library, not this file.

require_relative "cli/command"
require_relative "cli/parser"

module Libs
  # A program its user runs from a shell: one command per rule with `@on(cli("name"))`, its parameters the
  # arguments, every state kept in the `@store` file between runs. Everything is read from the app's manifest
  # (`PROGRAM.apps`) and the declared types of the rules' parameters and of the states.
  module Cli
    OK = 0
    REFUSED = 1
    USAGE = 2
    STORE = 3

    # Help and usage errors are answered from the rules' parameters alone; the store is opened only once a
    # command is known to run. Returns the exit code.
    def self.run(program, app, argv, out:, err:, env:, **)
      table = Table.new(program, app)
      args = argv.map { |arg| arg.dup.force_encoding(Encoding::UTF_8) }
      return say(err, table.help, USAGE) if args.empty?
      return say(out, table.help, OK) if Parser.help?(args)

      invocation = Parser.parse(table, args)
      require_relative "cli/dispatch"
      Dispatch.call(program, app, invocation, env:, out:, err:)
    rescue UsageError => e
      say(err, "error: #{e.message}\n", USAGE)
    end

    def self.say(stream, text, code)
      stream.write(text)
      code
    end
  end
end
