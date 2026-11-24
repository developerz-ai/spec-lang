# frozen_string_literal: true

# Projected by hand from the cli vocabulary: exit codes, error lines, output after commit (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "app"
require_relative "cli/exit_code"
require_relative "cli/usage_error"
require_relative "cli/help"
require_relative "cli/parser"

module Todo
  # The command line of the app: arguments in, lines and an exit code out.
  module CLI
    # Help and usage errors are answered from the parameter table alone: the board and the store
    # are loaded only once a command is known to run.
    def self.run(argv, env:, out:, err:)
      args = argv.map { |arg| arg.dup.force_encoding(Encoding::UTF_8) }
      return say(err, Help::TEXT, ExitCode::USAGE) if args.empty?
      return say(out, Help::TEXT, ExitCode::OK) if Parser.help?(args)

      invocation = Parser.parse(args)
      require_relative "cli/dispatch"
      Dispatch.call(invocation, path: App.store_path(env), out:, err:)
    rescue UsageError => e
      say(err, "error: #{e.message}\n", ExitCode::USAGE)
    end

    def self.say(stream, text, code)
      stream.write(text)
      code
    end
  end
end
