# frozen_string_literal: true

# Projected by hand from the cli vocabulary end to end: output, error lines, exit codes (docs/cli-vocabulary.md, docs/contract.md sections 2 and 3). Stand-in for spec-lang build output; change the spec, not this file.

require "test_helper"

class RunTest < Minitest::Test
  Result = Data.define(:out, :err, :code)

  def setup
    @dir = Dir.mktmpdir("todo-cli-")
    @path = File.join(@dir, "board.json")
  end

  def teardown = FileUtils.remove_entry(@dir)

  def todo(*argv, env: { "TODO_STORE" => @path })
    out = StringIO.new
    err = StringIO.new
    code = Todo::CLI.run(argv, env:, out:, err:)
    Result.new(out: out.string, err: err.string, code:)
  end

  def test_no_arguments_is_help_on_standard_error
    assert_equal Result.new(out: "", err: Todo::CLI::Help::TEXT, code: 2), todo
  end

  def test_help_is_on_standard_output_and_checks_nothing_else
    assert_equal Result.new(out: Todo::CLI::Help::TEXT, err: "", code: 0), todo("--help")
    assert_equal Result.new(out: Todo::CLI::Help::TEXT, err: "", code: 0), todo("frobnicate", "-h")
  end

  def test_a_usage_error_is_one_line_exit_two_and_touches_no_file
    assert_equal Result.new(out: "", err: "error: unknown command \"frobnicate\"\n", code: 2), todo("frobnicate")
    assert_equal Result.new(out: "", err: "error: missing <title>\n", code: 2), todo("add")
    assert_empty Dir.children(@dir)
  end

  def test_an_argument_is_printed_as_given
    assert_equal "error: unknown command \"café \\ \"x\"\"\n", todo("café \\ \"x\"").err
    assert_equal "error: unexpected argument \"\xFF\"\n".b, todo("clear", "\xFF".b).err.b
  end

  def test_rules_end_to_end
    assert_equal Result.new(out: "added 1\n", err: "", code: 0), todo("add", "milk")
    assert_equal "added 2\n", todo("add", "bread", "--tags", "shop", "--tags", "home", "--priority", "high").out
    assert_equal "2 [ ] bread !high #home #shop\n1 [ ] milk\n", todo("list").out
    assert_equal "1 [x] milk\n", todo("done", "1").out
    assert_equal "1 open, 1 done\n", todo("stats").out
    assert_equal "2 [ ] loaf !high #home #shop\n", todo("rename", "2", "loaf").out
    assert_equal "1 [ ] milk\n", todo("reopen", "01").out
    assert_equal "removed 1\n", todo("remove", "1").out
    assert_equal "cleared 0\n", todo("clear").out
    assert_equal %({"version":1,"next_no":3,"tasks":[{"no":2,"title":"loaf","status":"Open","priority":"High","tags":["home","shop"]}]}\n),
                 File.read(@path)
  end

  def test_a_refusal_is_its_name_exit_one_and_leaves_the_store
    todo("add", "a")
    todo("done", "1")
    before = File.read(@path)

    assert_equal Result.new(out: "", err: "error: AlreadyDone\n", code: 1), todo("done", "1")
    assert_equal Result.new(out: "", err: "error: NoTask\n", code: 1), todo("remove", "18446744073709551617")
    todo("reopen", "1")

    assert_equal Result.new(out: "", err: "error: NotDone\n", code: 1), todo("reopen", "1")
    assert_equal before.sub("Done", "Open"), File.read(@path)
  end

  def test_a_command_line_argument_is_text_whatever_the_locale_says
    assert_equal 0, todo("add", "café".b).code
    assert_equal "1 [ ] café\n", todo("list").out
  end

  def test_store_errors_exit_three
    File.write(@path, "{}")

    assert_equal Result.new(out: "", err: "error: store #{@path}: unreadable\n", code: 3), todo("stats")
    assert_equal Result.new(out: "", err: "error: store #{@path}: unreadable\n", code: 3), todo("add", "x")
    File.write(@path, %({"version":1,"next_no":#{2**53},"tasks":[]}\n))

    assert_equal Result.new(out: "", err: "error: store #{@path}: full\n", code: 3), todo("add", "x")
    missing = File.join(@dir, "lists", "home.json")

    assert_equal Result.new(out: "", err: "error: store #{missing}: unwritable\n", code: 3), todo("list", env: { "TODO_STORE" => missing })
  end

  def test_the_store_is_dot_todo_json_in_the_working_directory_unless_named
    Dir.chdir(@dir) do
      assert_equal 0, todo("add", "a", env: {}).code
      assert_equal 0, todo("add", "b", env: { "TODO_STORE" => "" }).code
      assert_equal 0, todo("add", "c", env: { "TODO_STORE" => "other.json" }).code
    end

    assert_equal %w[.todo.json .todo.json.lock other.json other.json.lock], Dir.children(@dir).sort
  end
end
