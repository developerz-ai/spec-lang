# frozen_string_literal: true

# Projected by hand from the input script the game vocabulary derives from Pad (docs/contract.md section 2, docs/game-vocabulary.md). Stand-in for spec-lang build output; change the spec, not this file.

require_relative "test_helper"

module Skyhop
  module TestSupport
    class ScriptTest < Minitest::Test
      Hold = Script::Hold

      def read(text) = Script.read(text.b)

      def refusal(text) = assert_raises(Script::Invalid) { read(text) }.message

      def test_comments_blank_lines_tabs_and_leading_zeros
        assert_equal [Hold.new(2, REST), Hold.new(3, RIGHT)], read("# wait\n\n  002\t-   # two\n0003 R\n")
      end

      def test_a_last_line_without_a_newline_counts
        assert_equal [Hold.new(4, REST), Hold.new(1, RIGHT)], read("4 -\n1 R")
      end

      def test_keys_in_any_order
        all = Pad.new(left: true, right: true, forward: true, back: true, jump: true)

        assert_equal [Hold.new(3, Pad.new(right: true, jump: true)), Hold.new(1, all)], read("3 JR\n1 BJFRL\n")
      end

      def test_empty_input
        assert_empty read("")
        assert_empty read("\n\n# nothing\n")
      end

      def test_refusals
        {
          "5\n" => "input line 1: expected <ticks> <keys>",
          "5 R J\n" => "input line 1: expected <ticks> <keys>",
          "0 R\n" => "input line 1: ticks must be 1 to 100000",
          "100001 R\n" => "input line 1: ticks must be 1 to 100000",
          "-3 R\n" => "input line 1: ticks must be 1 to 100000",
          "R 3\n" => "input line 1: ticks must be 1 to 100000",
          "1 r\n" => 'input line 1: unknown key "r"',
          "1 R-\n" => 'input line 1: unknown key "-"',
          "1 --\n" => 'input line 1: unknown key "-"',
          "1 RJR\n" => 'input line 1: key "R" twice',
          "1 RRZ\n" => 'input line 1: key "R" twice',
          "1 RZR\n" => 'input line 1: unknown key "Z"',
          "1 R\r\n" => "input line 1: unknown key \"\r\"",
          "# header\n\n3 R\n2 X\n0 R\n" => 'input line 4: unknown key "X"',
          "1 R\n1 ÿ\n" => "input line 2: unknown key \"ÿ\"",
          "#{"100000 R\n" * 10}1 R\n" => "input: more than 1000000 ticks"
        }.each { |text, message| assert_equal message.b, refusal(text).b, text.inspect }
      end

      def test_invalid_utf8_is_an_unknown_key_at_its_line
        assert_equal "input line 2: unknown key \"\xff\"".b, refusal("1 R\n1 R\xffJ\n").b
      end

      def test_a_million_ticks_is_allowed
        assert_equal 1_000_000, read("100000 R\n" * 10).sum(&:ticks)
      end
    end
  end
end
