# frozen_string_literal: true

# Projected by hand from the one lock around a rule (docs/contract.md section 3, "Lock"). Stand-in for spec-lang build output; change the spec, not this file.

require "test_helper"

class LockTest < Minitest::Test
  Lock = Todo::Store::Lock

  def setup
    @dir = Dir.mktmpdir("todo-lock-")
    @path = File.join(@dir, ".todo.json.lock")
  end

  def teardown = FileUtils.remove_entry(@dir)

  def hold(exclusive:, timeout: 0.05, &) = Lock.hold(@path, exclusive:, timeout:, interval: 0.01, &)

  def test_creates_the_lock_file_and_never_deletes_it
    assert_equal :held, hold(exclusive: true) { :held }
    assert_path_exists @path
    assert_equal "", File.read(@path)
  end

  def test_exclusive_shuts_out_everyone
    hold(exclusive: true) do
      assert_raises(Lock::Busy) { hold(exclusive: true) { flunk "two writers" } }
      assert_raises(Lock::Busy) { hold(exclusive: false) { flunk "a reader beside a writer" } }
    end
  end

  def test_shared_lets_readers_in_and_keeps_writers_out
    hold(exclusive: false) do
      assert_equal :second, hold(exclusive: false) { :second }
      assert_raises(Lock::Busy) { hold(exclusive: true) { flunk "a writer beside a reader" } }
    end
  end

  def test_released_after_the_block_even_when_it_raises
    assert_raises(RuntimeError) { hold(exclusive: true) { raise "boom" } }

    assert_equal :again, hold(exclusive: true) { :again }
  end

  def test_waits_for_a_lock_that_is_released_in_time
    File.open(@path, File::RDONLY | File::CREAT) do |holder|
      holder.flock(File::LOCK_EX)
      releaser = Thread.new do
        sleep 0.05
        holder.flock(File::LOCK_UN)
      end

      assert_equal :waited, hold(exclusive: true, timeout: 5.0) { :waited }
      releaser.join
    end
  end

  def test_gives_up_only_after_the_timeout
    started = Process.clock_gettime(Process::CLOCK_MONOTONIC)

    hold(exclusive: true) do
      assert_raises(Lock::Busy) { hold(exclusive: true, timeout: 0.1) { flunk "held twice" } }
    end

    assert_operator Process.clock_gettime(Process::CLOCK_MONOTONIC) - started, :>=, 0.1
  end

  def test_a_lock_file_that_cannot_be_created_is_unavailable
    assert_raises(Lock::Unavailable) { Lock.hold(File.join(@dir, "missing", "x.lock"), exclusive: false) { flunk "held" } }
  end

  def test_the_contract_waits_five_seconds_in_ten_millisecond_steps
    assert_in_delta 5.0, Lock::TIMEOUT
    assert_in_delta 0.01, Lock::INTERVAL
  end
end
