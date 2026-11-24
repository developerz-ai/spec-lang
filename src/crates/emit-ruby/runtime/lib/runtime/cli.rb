require_relative "cli/command"
require_relative "cli/parser"

module Runtime
  module CLI
    # Help and usage errors are answered from the parameter table alone: the rules and the store
    # are loaded only once a command is known to run.
    def self.run(app, argv, env:, out:, err:)
      args = argv.map { |arg| arg.dup.force_encoding(Encoding::UTF_8) }
      return say(err, app.help, ExitCode::USAGE) if args.empty?
      return say(out, app.help, ExitCode::OK) if Parser.help?(args)

      invocation = Parser.parse(app, args)
      require_relative "cli/dispatch"
      Dispatch.call(app, invocation, path: app.store_path(env), out:, err:)
    rescue UsageError => e
      say(err, "error: #{e.message}\n", ExitCode::USAGE)
    end

    def self.say(stream, text, code)
      stream.write(text)
      code
    end
  end
end
