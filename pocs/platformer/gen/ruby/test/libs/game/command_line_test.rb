# frozen_string_literal: true

# The Ruby runtime of the `game` library (libs/game/real/ruby): tests of the command line and the script, on the
# app it was built into.
# The same in every app with the game shape; change the library, not this file.

require "minitest/autorun"
require "stringio"
require "app"
require "libs/game/commands"
require "libs/game/fnv"

module Libs
  module Game
    class CommandLineTest < Minitest::Test
      def shape = Shape.new(PROGRAM, App::MANIFEST)

      def run_app(*argv, stdin: "")
        out = StringIO.new
        err = StringIO.new
        code = Game.run(PROGRAM, App::MANIFEST, argv, stdin: StringIO.new(stdin), out:, err:, env: {})
        [code, out.string, err.string]
      end

      def test_no_arguments_is_help_on_standard_error
        assert_equal [USAGE, "", shape.help], run_app
      end

      def test_help_anywhere_is_help_on_standard_output
        assert_equal [OK, shape.help, ""], run_app("run", "-h")
        %w[play run draw shot].each { |name| assert_includes shape.help, "\n  #{name}" }
      end

      def test_the_first_problem_from_the_left_is_reported
        assert_equal [USAGE, "", "error: unknown command \"fly\"\n"], run_app("fly")
        assert_equal [USAGE, "", "error: unknown flag --out\n"], run_app("run", "--out", "x")
        assert_equal [USAGE, "", "error: --at needs a value\n"], run_app("draw", "--at")
        assert_equal [USAGE, "", "error: --at given twice\n"], run_app("draw", "--at", "1", "--at", "2")
        assert_equal [USAGE, "", "error: --at: not a tick\n"], run_app("draw", "--at", "-1")
        assert_equal [USAGE, "", "error: --out: not a path\n"], run_app("shot", "--out", "")
        assert_equal [USAGE, "", "error: unexpected argument \"x\"\n"], run_app("run", "x")
      end

      def test_a_script_line_is_ticks_then_keys
        key = shape.keys.keys.first
        holds = Script.read(shape, "2 #{key}  # held\n\n1 -")
        assert_equal [2, 1], holds.map(&:ticks)
        assert_equal [true, false], holds.map { |hold| hold.pad.deconstruct.first }
      end

      def test_a_bad_script_names_its_line_and_stops_before_the_first_tick
        key = shape.keys.keys.first
        {
          "1\n" => "input line 1: expected <ticks> <keys>",
          "\n0 -\n" => "input line 2: ticks must be 1 to 100000",
          "1 #{key}#{key}\n" => "input line 1: key \"#{key}\" twice",
          "1 #{key}?\n" => "input line 1: unknown key \"?\"",
          "1 -#{key}\n" => "input line 1: unknown key \"-\"",
          "100000 -\n" * 11 => "input: more than 1000000 ticks"
        }.each do |script, message|
          assert_equal [USAGE, "", "error: #{message}\n"], run_app("run", stdin: script)
        end
      end

      def test_a_run_ends_with_the_report_and_never_reads_the_keyboard
        code, out, err = run_app("run", stdin: "3 -\n")
        assert_equal [OK, ""], [code, err]
        assert out.end_with?("\n")
        refute_empty out.lines
      end

      def test_draw_stops_at_the_tick_asked_for
        early = run_app("draw", "--at", "0", stdin: "5 -\n")
        late = run_app("draw", stdin: "5 -\n")
        assert_equal [OK, ""], [early[0], early[2]]
        assert_equal run_app("draw", "--at", "0", stdin: ""), early
        assert_equal run_app("draw", "--at", "5", stdin: "9 -\n"), late
      end

      def test_shot_without_a_display_cannot_open_a_window
        assert_equal [WINDOW, "", "error: window: cannot open\n"], run_app("shot", stdin: "1 -\n")
      end

      def test_the_fast_hash_is_fnv_1a
        ["", "a", "foobar", (0..255).map(&:chr).join * 9].each do |bytes|
          assert_equal FNV.reference(bytes.b), FNV.hex(bytes.b)
        end
        assert_equal "cbf29ce484222325", FNV.hex("")
        assert_equal "af63dc4c8601ec8c", FNV.hex("a")
      end
    end
  end
end
