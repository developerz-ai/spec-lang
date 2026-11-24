# frozen_string_literal: true

# Projected by hand from the command line the game vocabulary derives for app SkyhopRuby (docs/contract.md section 2, src/apps/ruby.slg). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "../test_helper"

module Skyhop
  module TestSupport
    class ParserTest < Minitest::Test
      Command = CLI::Command
      Parser = CLI::Parser

      ACCEPTED = [
        [[], :help_to_err],
        [%w[--help], :help],
        [%w[run --level 9 -h], :help],
        [%w[draw --at -h], :help],
        [%w[jump --help], :help],
        [%w[run], Command.new("run", 1, nil, nil)],
        [%w[play --level 2], Command.new("play", 2, nil, nil)],
        [%w[run --level 001], Command.new("run", 1, nil, nil)],
        [%w[draw --at 0], Command.new("draw", 1, 0, nil)],
        [%w[draw --at 000123456789012345678901234567890], Command.new("draw", 1, 123_456_789_012_345_678_901_234_567_890, nil)],
        [%w[shot --out --level --level 2 --at 5], Command.new("shot", 2, 5, "--level")]
      ].freeze

      REFUSED = [
        [%w[jump], 'unknown command "jump"'],
        [%w[--level], 'unknown command "--level"'],
        [%w[run --fast], "unknown flag --fast"],
        [%w[run --level=2], "unknown flag --level=2"],
        [%w[run --at 3], "unknown flag --at"],
        [%w[play --out x.png], "unknown flag --out"],
        [%w[run --level], "--level needs a value"],
        [%w[draw --at 1 --at 2], "--at given twice"],
        [%w[draw --level 1 --level], "--level needs a value"],
        [%w[run --level 0], "--level: not a LevelNo"],
        [%w[run --level 3], "--level: not a LevelNo"],
        [%w[run --level -1], "--level: not a LevelNo"],
        [%w[run --level one], "--level: not a LevelNo"],
        [["run", "--level", ""], "--level: not a LevelNo"],
        [%w[run --level 99999999999999999999999], "--level: not a LevelNo"],
        [["run", "--level", "٢"], "--level: not a LevelNo"],
        [%w[draw --at -1], "--at: not a tick"],
        [%w[draw --at 1.5], "--at: not a tick"],
        [["draw", "--at", ""], "--at: not a tick"],
        [["shot", "--out", ""], "--out: not a path"],
        [%w[run meadow], 'unexpected argument "meadow"'],
        [%w[draw -], 'unexpected argument "-"'],
        [%w[run --level 7 --fast], "--level: not a LevelNo"],
        [%w[run --fast --level 7], "unknown flag --fast"]
      ].freeze

      def test_accepted
        ACCEPTED.each { |argv, expected| assert_equal expected, Parser.parse(argv), argv.inspect }
      end

      def test_refused
        REFUSED.each do |argv, message|
          error = assert_raises(Parser::Usage, argv.inspect) { Parser.parse(argv) }

          assert_equal message, error.message, argv.inspect
        end
      end

      def test_help_is_the_contract_text
        contract = File.read(File.join(POC, "docs", "contract.md"))
        text = contract[/### Help\n.*?```text\n(.*?)```/m, 1]

        assert_equal text, CLI::HELP
      end

      def test_exit_codes_and_streams
        [[[], 2, "", CLI::HELP], [%w[-h], 0, CLI::HELP, ""], [%w[run x], 2, "", "error: unexpected argument \"x\"\n"]]
          .each do |argv, code, out, err|
            streams = [StringIO.new, StringIO.new]

            assert_equal code, CLI.run(argv, stdin: StringIO.new, out: streams[0], err: streams[1], env: {})
            assert_equal [out, err], streams.map(&:string), argv.inspect
          end
      end
    end
  end
end
