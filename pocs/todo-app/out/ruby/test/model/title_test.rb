# frozen_string_literal: true

# Projected by hand from @cases on fn is_title (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

require "test_helper"
require "support/spec_oracle"

class TitleTest < Minitest::Test
  CASES = { "milk" => true, "" => false, "   " => false, "a\tb" => false, "　" => false,
            "a\u2028b" => false, "a\u0085b" => false }.freeze

  def test_is_title_cases
    CASES.each { |text, expected| assert_equal expected, Todo::Model.title?(text), text.inspect }
  end

  def test_size_counts_code_points
    assert Todo::Model.title?("é" * 200)
    refute Todo::Model.title?("é" * 201)
    assert Todo::Model.title?("\u{1F600}" * 200)
    refute Todo::Model.title?("\u{1F600}" * 201)
    assert Todo::Model.title?("x" * 200)
    refute Todo::Model.title?("x" * 201)
  end

  def test_trim_is_exactly_unicode_white_space
    Todo::TestSupport::SpecOracle::WHITE_SPACE.each do |point|
      refute Todo::Model.title?([point].pack("U")), format("U+%04X is white space", point)
    end
    assert_equal 25, Todo::TestSupport::SpecOracle::WHITE_SPACE.size
    ["﻿", "​", "᠎"].each { |text| assert_operator Todo::Model, :title?, text }
  end

  def test_control_bytes_and_line_breaks_are_refused_but_other_c1_controls_inside_are_kept
    refute Todo::Model.title?("a\nb")
    refute Todo::Model.title?("a\u007Fb")
    refute Todo::Model.title?("a\u0000b")
    refute Todo::Model.title?("a\u2029b")
    assert Todo::Model.title?("a\u2027b\u0084c")
  end

  def test_text_that_is_not_unicode_is_not_a_title
    refute Todo::Model.title?("\xFF")
    refute Todo::Model.title?("milk".b)
    refute Todo::Model.title?(nil)
  end

  def test_agrees_with_the_spec_formula_on_random_text
    random = Random.new(7)
    alphabet = [*Todo::TestSupport::SpecOracle::WHITE_SPACE, 0, 9, 31, 127, 0x61, 0x7A, 0xE9, 0x180E, 0x200B, 0xFEFF, 0x1F600]
    2000.times do
      text = Array.new(random.rand(0..4)) { alphabet.sample(random: random) }.pack("U*") * [1, 1, 1, 70, 210].sample(random: random)

      assert_equal Todo::TestSupport::SpecOracle.title?(text), Todo::Model.title?(text), text.inspect
    end
  end
end
