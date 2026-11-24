# frozen_string_literal: true

# Projected by hand from @cases on fn is_tag (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

require "test_helper"
require "support/spec_oracle"

class TagTest < Minitest::Test
  CASES = { "home" => true, "deep-work" => true, "Home" => false, "" => false, "a b" => false,
            "-x" => false, "--all" => false }.freeze

  def test_is_tag_cases
    CASES.each { |text, expected| assert_equal expected, Todo::Model.tag?(text), text.inspect }
  end

  def test_size_bounds
    assert Todo::Model.tag?("a" * 32)
    refute Todo::Model.tag?("a" * 33)
    assert Todo::Model.tag?("0-9")
  end

  def test_a_trailing_newline_is_not_part_of_a_tag
    refute Todo::Model.tag?("home\n")
    refute Todo::Model.tag?("é")
    refute Todo::Model.tag?("\xFF")
  end

  def test_agrees_with_the_spec_formula_on_random_text
    random = Random.new(11)
    alphabet = ["a", "z", "0", "9", "-", "A", " ", "\n", "_", "é", "{", "/", ":", "`"]
    2000.times do
      text = Array.new(random.rand(0..34)) { alphabet.sample(random: random) }.join
      text = "a" * random.rand(30..34) if random.rand(4).zero?

      assert_equal Todo::TestSupport::SpecOracle.tag?(text), Todo::Model.tag?(text), text.inspect
    end
  end
end
