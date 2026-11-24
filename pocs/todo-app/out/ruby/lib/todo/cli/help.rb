# frozen_string_literal: true

# Projected by hand from the --help line of every cli rule (docs/cli-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "../app"
require_relative "commands"

module Todo
  module CLI
    module Help
      TEXT = <<~HELP.freeze
        usage: #{App::BIN} <command> [args]

        commands:
        #{Commands::ALL.map { |command| "  #{command.usage}" }.join("\n")}
      HELP
    end
  end
end
