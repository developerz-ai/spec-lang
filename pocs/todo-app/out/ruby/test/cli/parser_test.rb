# frozen_string_literal: true

# Projected by hand from the parsing order of the cli vocabulary (docs/contract.md section 2, "Parsing, in this order"). Stand-in for spec-lang build output; change the spec, not this file.

require "test_helper"

class ParserTest < Minitest::Test
  Parser = Todo::CLI::Parser

  def args(*argv) = Parser.parse(argv).args

  def error(*argv) = assert_raises(Todo::CLI::UsageError, argv.inspect) { Parser.parse(argv) }.message

  def test_help_is_asked_for_anywhere_before_the_first_double_dash
    assert Parser.help?(%w[--help])
    assert Parser.help?(%w[add -h])
    assert Parser.help?(%w[add x --tags -h])
    assert Parser.help?(%w[frobnicate --nope --help])
    refute Parser.help?(%w[add -- --help])
    refute Parser.help?(%w[add -help])
    refute Parser.help?(%w[add --help=yes])
  end

  def test_every_parameter_is_present_with_its_default
    assert_equal({ title: "x", priority: :Normal, tags: Set[] }, args("add", "x"))
    assert_equal({ all: false, tag: nil }, args("list"))
    assert_equal({ no: 7 }, args("done", "007"))
    assert_equal({ no: 3, title: "new" }, args("rename", "3", "new"))
    assert_empty args("clear")
    assert_empty args("stats")
  end

  def test_the_command_is_named
    assert_equal "add", Parser.parse(%w[add x]).command.name
    assert_equal :add, Parser.parse(%w[add x]).command.rule
  end

  def test_flags_in_any_place
    assert_equal({ title: "ship it", priority: :High, tags: Set["work"] }, args("add", "--priority", "high", "ship it", "--tags", "work"))
    assert_equal({ all: true, tag: "home" }, args("list", "--tag", "home", "--all"))
  end

  def test_repeated_tags_collapse
    assert_equal Set["a", "b"], args("add", "x", "--tags", "b", "--tags", "a", "--tags", "b")[:tags]
  end

  def test_double_dash_ends_flags_once
    assert_equal "--all", args("add", "--", "--all")[:title]
    assert_equal "--", args("add", "--", "--")[:title]
    assert_equal "-x", args("add", "-x")[:title]
    assert_equal "-", args("add", "-")[:title]
    assert_equal({ title: "x", priority: :Low, tags: Set[] }, args("add", "--priority", "low", "--", "x"))
  end

  def test_a_value_flag_takes_the_next_argument_whatever_it_is
    assert_equal "--priority: not a Priority", error("add", "x", "--priority", "--tags")
    assert_equal "--tag: not a Tag", error("list", "--tag", "--all")
    assert_equal "--tags: not a Tag", error("add", "x", "--tags", "--")
    assert_equal "missing <title>", error("add", "--tags", "a", "--priority", "low")
  end

  def test_a_number_of_any_length_is_a_task_no
    assert_equal 99_999_999_999_999_999_999_999_999_999_999, args("done", "99999999999999999999999999999999")[:no]
  end

  USAGE_ERRORS = {
    %w[frobnicate] => 'unknown command "frobnicate"',
    %w[Add x] => 'unknown command "Add"',
    %w[-- add] => 'unknown command "--"',
    ["", "x"] => 'unknown command ""',
    %w[add] => "missing <title>",
    %w[done] => "missing <no>",
    %w[rename 1] => "missing <title>",
    %w[rename] => "missing <no>",
    %w[add buy milk] => 'unexpected argument "milk"',
    %w[clear now] => 'unexpected argument "now"',
    %w[stats --] => nil,
    %w[stats -- --] => 'unexpected argument "--"',
    %w[done 0] => "<no>: not a TaskNo",
    %w[done 000] => "<no>: not a TaskNo",
    %w[done -1] => "<no>: not a TaskNo",
    %w[done +1] => "<no>: not a TaskNo",
    %w[done 1.5] => "<no>: not a TaskNo",
    %w[done 1_000] => "<no>: not a TaskNo",
    %w[done 0x1] => "<no>: not a TaskNo",
    %w[done abc] => "<no>: not a TaskNo",
    ["done", ""] => "<no>: not a TaskNo",
    ["done", " 1"] => "<no>: not a TaskNo",
    %W[done 1\n] => "<no>: not a TaskNo",
    %w[done ١] => "<no>: not a TaskNo",
    ["done", "1\xFF"] => "<no>: not a TaskNo",
    ["add", ""] => "<title>: not a Title",
    %W[add a\tb] => "<title>: not a Title",
    %W[add a\u2028b] => "<title>: not a Title",
    %W[add a\u2029b] => "<title>: not a Title",
    %W[add a\u0085b] => "<title>: not a Title",
    ["add", "\xFF"] => "<title>: not a Title",
    %w[rename 1 x y] => 'unexpected argument "y"',
    %w[add x --force] => "unknown flag --force",
    %w[add x --priority=high] => "unknown flag --priority=high",
    %w[add x ---] => "unknown flag ---",
    %w[add x --tag a] => "unknown flag --tag",
    %w[done 1 --all] => "unknown flag --all",
    %w[add x --priority] => "--priority needs a value",
    %w[add x --tags] => "--tags needs a value",
    %w[list --tag] => "--tag needs a value",
    %w[add x --priority High] => "--priority: not a Priority",
    %w[add x --priority Normal] => "--priority: not a Priority",
    ["add", "x", "--priority", ""] => "--priority: not a Priority",
    %w[add x --tags Home] => "--tags: not a Tag",
    ["add", "x", "--tags", ""] => "--tags: not a Tag",
    ["add", "x", "--tags", "a" * 33] => "--tags: not a Tag",
    %w[add x --tags -x] => "--tags: not a Tag",
    %w[add x --tags --all] => "--tags: not a Tag",
    %w[list --tag Home] => "--tag: not a Tag",
    %w[list --tag --all] => "--tag: not a Tag",
    %w[list --all --all] => "--all given twice",
    %w[list --tag a --tag b] => "--tag given twice",
    %w[list --tag a --tag B] => "--tag given twice",
    %w[list --tag a --tag] => "--tag needs a value",
    %w[add x --priority low --priority high] => "--priority given twice",
    %w[done abc --force] => "<no>: not a TaskNo",
    %w[done --force abc] => "unknown flag --force",
    %w[add x y --priority nope] => 'unexpected argument "y"',
    %w[add --priority nope] => "--priority: not a Priority",
    %w[rename x] => "<no>: not a TaskNo"
  }.freeze

  def test_the_first_problem_from_the_left_is_the_usage_error
    USAGE_ERRORS.each do |argv, message|
      message.nil? ? Parser.parse(argv) : assert_equal(message, error(*argv), argv.inspect)
    end
  end
end
