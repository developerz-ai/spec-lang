# frozen_string_literal: true

# Projected by hand from app SkyhopRuby(start, tick, input, frame, dump, report) and @bin("skyhop") (src/apps/ruby.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "cli/help"
require_relative "cli/parser"

module Skyhop
  # The command line: parse, then hand the command to the app. Exit codes per contract section 2.
  module CLI
    OK = 0
    USAGE = 2
    WINDOW = 3

    module_function

    def run(argv, stdin:, out:, err:, env:)
      command = Parser.parse(argv.map(&:b))
      case command
      when :help_to_err then err.write(HELP) && USAGE
      when :help then out.write(HELP) && OK
      else
        require_relative "app"
        App.run(command, stdin:, out:, err:, env:)
      end
    rescue Parser::Usage => e
      err.write("error: #{e.message}\n")
      USAGE
    end
  end
end
