# frozen_string_literal: true

# Projected by hand from test three_open_one_done, numbers_start_at_one, numbers_are_never_reused, empty_board_says_so, list_ranks_urgent_first, list_all_and_by_tag, done_and_reopen_move_once, rename_keeps_the_rest, clear_removes_only_done, unknown_numbers_are_refused (src/todo/tests/scenarios.slg). Stand-in for spec-lang build output; change the spec, not this file.

require "test_helper"
require "support/board_session"

class ScenariosTest < Minitest::Test
  Board = Todo::Board
  Task = Todo::Model::Task

  # @given(board.tasks -> Map(...), board.next_no -> 5)
  THREE_OPEN_ONE_DONE = Board::World.new(
    tasks: {
      1 => Task.new(no: 1, title: "low one", priority: :Low),
      2 => Task.new(no: 2, title: "plain"),
      3 => Task.new(no: 3, title: "urgent", priority: :High, tags: Set["work"]),
      4 => Task.new(no: 4, title: "finished", status: :Done, tags: Set["work", "home"])
    },
    next_no: 5
  )

  def three_open_one_done = Todo::TestSupport::BoardSession.new(THREE_OPEN_ONE_DONE)

  def empty_board = Todo::TestSupport::BoardSession.new

  def test_three_open_one_done
    board = three_open_one_done

    assert_equal [1, 2, 3, 4], board.world.tasks.keys
    assert_equal 5, board.world.next_no
    assert_equal "", board.out_text
  end

  def test_numbers_start_at_one
    board = empty_board

    assert_equal 1, board.call(:add, title: "milk")
    assert_equal 2, board.call(:add, title: "bread", tags: Set["shop", "home"])
    assert_equal "added 1\nadded 2\n", board.out_text
  end

  def test_numbers_are_never_reused
    board = empty_board

    assert_equal 1, board.call(:add, title: "a")
    assert_equal 2, board.call(:add, title: "b")
    board.call(:remove, no: 2)

    assert_equal 3, board.call(:add, title: "c")
  end

  def test_empty_board_says_so
    board = empty_board
    board.call(:list)
    board.call(:stats)

    assert_equal "nothing to do\n0 open, 0 done\n", board.out_text
  end

  def test_list_ranks_urgent_first
    board = three_open_one_done
    board.call(:list)

    assert_equal "3 [ ] urgent !high #work\n2 [ ] plain\n1 [ ] low one !low\n", board.out_text
  end

  def test_list_all_and_by_tag
    board = three_open_one_done
    board.call(:list, all: true, tag: "work")

    assert_equal "3 [ ] urgent !high #work\n4 [x] finished #home #work\n", board.out_text
  end

  def test_done_and_reopen_move_once
    board = three_open_one_done
    board.call(:done, no: 2)
    assert_raises(Board::AlreadyDone) { board.call(:done, no: 2) }
    board.call(:reopen, no: 2)
    assert_raises(Board::NotDone) { board.call(:reopen, no: 2) }

    assert_equal "2 [x] plain\n2 [ ] plain\n", board.out_text
  end

  def test_rename_keeps_the_rest
    board = three_open_one_done
    board.call(:rename, no: 3, title: "urgent, really")

    assert_equal Task.new(no: 3, title: "urgent, really", priority: :High, tags: Set["work"]), board.world.tasks[3]
  end

  def test_clear_removes_only_done
    board = three_open_one_done

    assert_equal 1, board.call(:clear)
    board.call(:stats)

    assert_equal "cleared 1\n3 open, 0 done\n", board.out_text
    refute board.world.tasks.key?(4)
  end

  # A property over no: TaskNo. Values are drawn small, at the edge, and past every machine word.
  def test_unknown_numbers_are_refused
    random = Random.new(Integer(ENV.fetch("TODO_SEED", "20261124")))
    numbers = [1, 4, 5, 6, 2**53, (2**64) + 1, 10**40] + Array.new(200) { random.rand(1..(10**random.rand(1..30))) }
    numbers.each do |no|
      next if no <= 4

      board = three_open_one_done
      assert_raises(Board::NoTask, "done #{no}") { board.call(:done, no:) }
      assert_raises(Board::NoTask, "reopen #{no}") { board.call(:reopen, no:) }
      assert_raises(Board::NoTask, "rename #{no}") { board.call(:rename, no:, title: "x") }
      assert_raises(Board::NoTask, "remove #{no}") { board.call(:remove, no:) }

      assert_equal "", board.out_text
      assert_equal THREE_OPEN_ONE_DONE, board.world
    end
  end
end
