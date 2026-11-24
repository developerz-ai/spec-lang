# frozen_string_literal: true

# Projected by hand from type Task, TaskNo, Status, Priority and their defaults (src/todo/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

require "test_helper"

class TaskTest < Minitest::Test
  Task = Todo::Model::Task

  def test_defaults
    task = Task.new(no: 1, title: "milk")

    assert_equal :Open, task.status
    assert_equal :Normal, task.priority
    assert_equal Set[], task.tags
  end

  def test_equality_is_structural
    assert_equal Task.new(no: 1, title: "milk", tags: Set["a", "b"]), Task.new(no: 1, title: +"milk", tags: Set["b", "a"])
    refute_equal Task.new(no: 1, title: "milk"), Task.new(no: 1, title: "milk", status: :Done)
  end

  def test_a_task_is_deeply_frozen_and_owns_its_values
    tags = Set[+"home"]
    task = Task.new(no: 1, title: +"milk", tags:)
    tags << "work"

    assert_predicate task, :frozen?
    assert_predicate task.title, :frozen?
    assert_predicate task.tags, :frozen?
    assert_equal Set["home"], task.tags
    assert(task.tags.all?(&:frozen?))
  end

  def test_every_claim_is_checked_by_the_constructor
    [
      { no: 0 }, { no: "1" }, { no: 1.0 }, { title: " " }, { title: "a\tb" }, { title: :milk }, { status: :open },
      { priority: "High" }, { tags: ["home"] }, { tags: Set["Home"] }, { tags: Set[1] }
    ].each do |change|
      assert_raises(Todo::Model::Invalid, change.inspect) { Task.new(no: 1, title: "milk", **change) }
    end
  end

  def test_with_checks_the_claims_again
    task = Task.new(no: 1, title: "milk")

    assert_equal "oat milk", task.with(title: "oat milk").title
    assert_raises(Todo::Model::Invalid) { task.with(title: "") }
  end

  def test_a_task_number_has_no_upper_bound
    assert_equal 10**40, Task.new(no: 10**40, title: "milk").no
  end
end
