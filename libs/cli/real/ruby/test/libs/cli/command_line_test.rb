# frozen_string_literal: true

# The Ruby runtime of the `cli` library (libs/cli/real/ruby): tests of the command line, on the app it was
# built into.
# The same in every app that uses `cli`; change the library, not this file.

require "minitest/autorun"
require "stringio"
require "tmpdir"
require "app"

module Libs
  module Cli
    class CommandLineTest < Minitest::Test
      def run_app(*argv, env: {})
        out = StringIO.new
        err = StringIO.new
        code = Cli.run(PROGRAM, App::MANIFEST, argv, out:, err:, env:)
        [code, out.string, err.string]
      end

      def table = Table.new(PROGRAM, App::MANIFEST)

      def test_no_arguments_is_help_on_standard_error
        code, out, err = run_app
        assert_equal [USAGE, "", table.help], [code, out, err]
      end

      def test_help_lists_every_command_once
        code, out, err = run_app("--help")
        assert_equal [OK, ""], [code, err]
        table.commands.each { |command| assert_equal 1, out.scan(/^  #{Regexp.escape(command.name)}( |$)/).size }
      end

      def test_help_wins_before_the_end_of_flags_only
        assert Parser.help?(["x", "-h"])
        refute Parser.help?(["x", "--", "--help"])
      end

      def test_an_unknown_command_is_a_usage_error
        code, out, err = run_app("no-such-command")
        assert_equal [USAGE, "", "error: unknown command \"no-such-command\"\n"], [code, out, err]
      end

      def test_an_unknown_flag_is_a_usage_error
        command = table.commands.first
        code, out, err = run_app(command.name, "--no-such-flag")
        assert_equal [USAGE, "", "error: unknown flag --no-such-flag\n"], [code, out, err]
      end

      def test_a_missing_positional_is_named
        command = table.commands.find { |c| !c.positionals.empty? } or skip("no command takes a positional argument")
        code, _, err = run_app(command.name)
        assert_equal [USAGE, "error: missing #{command.positionals.first.shown}\n"], [code, err]
      end

      def test_a_store_that_is_not_json_is_unreadable_and_left_alone
        command = table.commands.find { |c| c.positionals.empty? } or skip("every command takes an argument")
        Dir.mktmpdir do |dir|
          path = File.join(dir, "store.json")
          File.write(path, "not json\n")
          key = "#{App::MANIFEST.note("bin")["name"].upcase}_STORE"
          code, out, err = run_app(command.name, env: { key => path })
          assert_equal [STORE, "", "error: store #{path}: unreadable\n"], [code, out, err]
          assert_equal "not json\n", File.read(path)
        end
      end
    end
  end
end
