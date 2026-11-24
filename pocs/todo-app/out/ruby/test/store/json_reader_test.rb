# frozen_string_literal: true

# Projected by hand from the store format as read: any valid JSON, no key twice (docs/contract.md section 3, "Accepted on read"). Stand-in for spec-lang build output; change the spec, not this file.

require "test_helper"

class JsonReaderTest < Minitest::Test
  Reader = Todo::Store::JsonReader

  def test_reads_every_kind_of_value
    assert_equal({ "a" => [1, -2, true, false, nil, "x"], "b" => {} }, Reader.parse(%( { "a" : [ 1, -2, true,\tfalse, null, "x" ],\r\n"b":{} } )))
    assert_equal [], Reader.parse("[]")
  end

  def test_integers_are_exact_at_any_size
    assert_equal 9_007_199_254_740_993, Reader.parse("9007199254740993")
    assert_equal 10**40, Reader.parse("1#{"0" * 40}")
    assert_equal 0, Reader.parse("-0")
  end

  def test_a_fraction_or_exponent_is_not_an_integer
    ["1.0", "1e0", "1E+2", "-0.5", "1e999"].each do |literal|
      assert_equal Reader::Real.new(literal:), Reader.parse(literal)
    end
  end

  def test_strings_take_escapes_and_surrogate_pairs
    assert_equal "café \u{1F600} \"q\" \\ / \b\f\n\r\t", Reader.parse('"café 😀 \"q\" \\\\ \/ \b\f\n\r\t"')
    assert_equal "  raw \u007F", Reader.parse("\"  raw \u007F\"")
    assert_equal Encoding::UTF_8, Reader.parse('"a"').encoding
  end

  INVALID = {
    "empty" => "", "blank" => " \n", "bare word" => "todo", "trailing garbage" => "[] x", "two values" => "1 2",
    "duplicate key" => '{"a":1,"a":1}', "duplicate key written two ways" => '{"a":1,"a":2}',
    "trailing comma in array" => "[1,]", "trailing comma in object" => '{"a":1,}', "missing colon" => '{"a" 1}',
    "key not a string" => "{1:2}", "single quotes" => "['a']", "unclosed array" => "[1", "unclosed object" => '{"a":1',
    "unclosed string" => '"a', "leading zero" => "01", "plus sign" => "+1", "bare fraction" => ".5",
    "no digits after point" => "1.", "no digits in exponent" => "1e", "hex" => "0x10", "NaN" => "NaN",
    "raw newline in string" => "\"a\nb\"", "raw tab in string" => "\"a\tb\"", "unknown escape" => '"\x41"',
    "short unicode escape" => '"\u12"', "lone high surrogate" => '"\ud800"', "lone low surrogate" => '"\udc00"',
    "high surrogate then text" => '"\ud800x"', "high then high" => '"\ud800\ud800"', "comment" => "[] // x",
    "byte order mark" => "﻿[]", "capital literal" => "True", "invalid UTF-8" => "\"\xFF\"",
    "not tagged UTF-8" => "[]".b, "nested too deep" => "#{"[" * 40}#{"]" * 40}"
  }.freeze

  def test_everything_else_is_invalid
    INVALID.each { |name, text| assert_raises(Reader::Invalid, name) { Reader.parse(text) } }
  end

  def test_hostile_nesting_does_not_exhaust_the_stack
    assert_raises(Reader::Invalid) { Reader.parse("[" * 1_000_000) }
  end
end
