# frozen_string_literal: true

# Projected by hand from @store(file(".todo.json")): load, lock, atomic replace (docs/cli-vocabulary.md, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

require "test_helper"

class FileStoreTest < Minitest::Test
  Store = Todo::Store
  Board = Todo::Board

  ONE_TASK = %({"version":1,"next_no":2,"tasks":[{"no":1,"title":"a","status":"Open","priority":"Normal","tags":[]}]}\n)

  def setup
    @dir = Dir.mktmpdir("todo-store-")
    @path = File.join(@dir, ".todo.json")
    @store = Store::FileStore.new(@path, lock_timeout: 0.05)
  end

  def teardown
    FileUtils.chmod_R(0o755, @dir)
    FileUtils.remove_entry(@dir)
  end

  def entries = Dir.children(@dir).sort

  def add(title) = @store.transact(:write) { |world| Board.add(world, title:) }

  def test_a_missing_store_is_the_initial_world
    seen = nil
    @store.transact(:read) { |world| Board.list(seen = world) }

    assert_equal Board::World.initial, seen
  end

  def test_a_read_creates_the_lock_but_never_the_store
    @store.transact(:read) { |world| Board.stats(world) }

    assert_equal [".todo.json.lock"], entries
  end

  def test_a_write_replaces_the_store_and_leaves_no_temporary_file
    assert_equal 1, add("a").reply
    assert_equal ONE_TASK, File.read(@path)
    assert_equal [".todo.json", ".todo.json.lock"], entries
    assert_equal 2, add("b").reply
    assert_equal [".todo.json", ".todo.json.lock"], entries
  end

  def test_a_read_never_rewrites_a_store_that_is_not_canonical
    spaced = %( {"tasks":[],"next_no":4,"version":1} )
    File.write(@path, spaced)
    @store.transact(:read) { |world| Board.list(world) }

    assert_equal spaced, File.read(@path)
  end

  def test_a_write_makes_the_store_canonical
    File.write(@path, %( {"tasks":[],"next_no":4,"version":1} ))
    @store.transact(:write) { |world| Board.clear(world) }

    assert_equal %({"version":1,"next_no":4,"tasks":[]}\n), File.read(@path)
  end

  def test_a_write_of_the_bytes_already_there_is_skipped
    File.write(@path, ONE_TASK)
    before = File.stat(@path).ino
    @store.transact(:write) { |world| Board.rename(world, no: 1, title: "a") }

    assert_equal before, File.stat(@path).ino
    assert_equal ONE_TASK, File.read(@path)
  end

  def test_a_write_on_a_missing_store_creates_it_even_when_nothing_changed
    @store.transact(:write) { |world| Board.clear(world) }

    assert_equal %({"version":1,"next_no":1,"tasks":[]}\n), File.read(@path)
  end

  def test_a_refusal_writes_nothing_and_releases_the_lock
    File.write(@path, ONE_TASK)
    before = File.stat(@path).ino

    assert_raises(Board::NoTask) { @store.transact(:write) { |world| Board.done(world, no: 9) } }
    assert_equal before, File.stat(@path).ino
    assert_equal 2, add("b").reply
  end

  def test_counts_are_read_without_the_world
    File.write(@path, ONE_TASK)
    seen = nil
    outcome = @store.transact(:read, reads: :counts) { |counts| Board.stats(seen = counts) }

    assert_equal Board::Counts.new(open: 1, size: 1), seen
    assert_equal ["1 open, 0 done"], outcome.lines
  end

  def test_an_unreadable_store_names_the_path_as_configured
    File.write(@path, "{}")
    %i[world counts].each do |reads|
      error = assert_raises(Store::Unreadable) { @store.transact(:read, reads:) { flunk "ran the rule" } }

      assert_equal "store #{@path}: unreadable", error.message
    end
    assert_raises(Store::Unreadable) { add("x") }
    assert_equal "{}", File.read(@path)
  end

  def test_a_store_that_is_a_directory_is_unreadable
    Dir.mkdir(@path)

    assert_raises(Store::Unreadable) { @store.transact(:read) { flunk "ran the rule" } }
  end

  def test_a_lock_that_cannot_be_created_is_unwritable
    store = Store::FileStore.new(File.join(@dir, "lists", "home.json"))
    error = assert_raises(Store::Unwritable) { store.transact(:read) { flunk "ran the rule" } }

    assert_equal "store #{File.join(@dir, "lists", "home.json")}: unwritable", error.message
  end

  def test_a_directory_that_cannot_be_written_is_unwritable
    skip "the super-user writes anywhere" if Process.uid.zero?
    File.write("#{@path}.lock", "")
    File.chmod(0o555, @dir)

    assert_raises(Store::Unwritable) { add("a") }
    assert_equal [".todo.json.lock"], entries
  end

  def test_full_when_next_no_would_pass_two_to_the_53
    last = %({"version":1,"next_no":#{2**53},"tasks":[]}\n)
    File.write(@path, last)
    error = assert_raises(Store::Full) { add("a") }

    assert_equal "store #{@path}: full", error.message
    assert_equal last, File.read(@path)
  end

  def test_contended_when_another_holds_the_lock
    add("a")
    File.open("#{@path}.lock") do |other|
      other.flock(File::LOCK_EX)

      assert_raises(Store::Contended) { add("b") }
      assert_raises(Store::Contended) { @store.transact(:read) { flunk "read beside a writer" } }
    end

    assert_equal ONE_TASK, File.read(@path)
  end

  def test_readers_share_the_lock
    add("a")
    File.open("#{@path}.lock") do |other|
      other.flock(File::LOCK_SH)

      assert_equal ["1 [ ] a"], @store.transact(:read) { |world| Board.list(world) }.lines
      assert_raises(Store::Contended) { add("b") }
    end
  end

  def test_concurrent_writers_lose_no_number
    store = Store::FileStore.new(@path)
    replies = Array.new(8) { |i| Thread.new { store.transact(:write) { |world| Board.add(world, title: "job #{i}") }.reply } }.map(&:value)

    assert_equal (1..8).to_a, replies.sort
    assert_equal ["8 open, 0 done"], store.transact(:read, reads: :counts) { |counts| Board.stats(counts) }.lines
  end
end
