# frozen_string_literal: true

# Projected by hand from app TodoRuby with @bin("todo"): the executable as a process (src/apps/ruby.slg, docs/contract.md sections 2 and 3). Stand-in for spec-lang build output; change the spec, not this file.

require "test_helper"
require "open3"

class ExeTest < Minitest::Test
  def setup = @dir = Dir.mktmpdir("todo-exe-")

  def teardown = FileUtils.remove_entry(@dir)

  def todo(*argv, env: {})
    Open3.capture3({ "TODO_STORE" => nil, "LANG" => "C" }.merge(env), Todo::TestSupport::EXE, *argv, chdir: @dir)
  end

  def test_help
    out, err, status = todo("--help")

    assert_equal [Todo::CLI::Help::TEXT, "", 0], [out, err, status.exitstatus]
  end

  def test_a_board_in_the_working_directory_under_the_c_locale
    assert_equal(["added 1\n", "", 0], todo("add", "café ☕").then { |out, err, status| [out, err, status.exitstatus] })
    assert_equal "1 [ ] café ☕\n".b, todo("list").first.b
    assert_equal(["", "error: NoTask\n", 1], todo("done", "2").then { |out, err, status| [out, err, status.exitstatus] })
    assert_equal(["", "error: missing <no>\n", 2], todo("done").then { |out, err, status| [out, err, status.exitstatus] })
    assert_equal %w[.todo.json .todo.json.lock], Dir.children(@dir).sort
  end

  def test_contended_after_five_seconds_is_not_waited_for_here_but_a_held_lock_delays_a_writer
    todo("add", "a")
    File.open(File.join(@dir, ".todo.json.lock")) do |lock|
      lock.flock(File::LOCK_EX)
      releaser = Thread.new do
        sleep 0.3
        lock.flock(File::LOCK_UN)
      end
      started = Process.clock_gettime(Process::CLOCK_MONOTONIC)
      out, = todo("add", "b")

      assert_equal "added 2\n", out
      assert_operator Process.clock_gettime(Process::CLOCK_MONOTONIC) - started, :>=, 0.3
      releaser.join
    end
  end

  def test_only_what_a_command_needs_is_loaded
    loaded = lambda do |*argv|
      script = 'require "todo/cli"; Todo::CLI.run(ARGV, env: ENV, out: File.open(File::NULL, "w"), err: File.open(File::NULL, "w")); ' \
               'puts $LOADED_FEATURES.grep(%r{/lib/todo/}).map { |path| path[%r{/lib/todo/(.*)\.rb}, 1] }'
      out, = Open3.capture3(RbConfig.ruby, "--disable-gems", "-I", File.join(Todo::TestSupport::ROOT, "lib"), "-e", script, *argv, chdir: @dir)
      out.lines(chomp: true)
    end

    refute(loaded.call("--help").any? { |file| file.start_with?("store/", "board/", "model/task", "cli/dispatch") })
    refute(loaded.call("add").any? { |file| file.start_with?("store/", "board/", "model/task", "cli/dispatch") })
    refute(loaded.call("stats").any? { |file| %w[store/encoder store/atomic_file render model/task].include?(file) || file.start_with?("board/add") })
    refute(loaded.call("list").intersect?(%w[store/encoder store/atomic_file board/add]))
    assert_includes loaded.call("add", "x"), "store/encoder"
  end
end
