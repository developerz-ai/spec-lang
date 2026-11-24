# frozen_string_literal: true

# Projected by hand from @cases on fn tag_text and fn line; fn mark, badge, urgency, rank (src/todo/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

require "test_helper"

class RenderTest < Minitest::Test
  Task = Todo::Model::Task
  Render = Todo::Render

  def test_tag_text_cases
    assert_equal "", Render.tag_text(Set[])
    assert_equal " #home #work", Render.tag_text(Set["work", "home"])
  end

  def test_line_cases
    assert_equal "1 [ ] milk", Render.line(Task.new(no: 1, title: "milk"))
    assert_equal "2 [x] ship !high #work",
                 Render.line(Task.new(no: 2, title: "ship", status: :Done, priority: :High, tags: Set["work"]))
  end

  def test_mark_and_badge_cover_every_variant
    assert_equal([" ", "x"], Todo::Model::Status::ALL.map { |status| Render.mark(status) })
    assert_equal([" !low", "", " !high"], Todo::Model::Priority::ALL.map { |priority| Render.badge(priority) })
  end

  def test_rank_is_urgency_then_number
    assert_equal [0, 9], Render.rank(Task.new(no: 9, title: "a", priority: :High))
    assert_equal [1, 2], Render.rank(Task.new(no: 2, title: "a"))
    assert_equal [2, 1], Render.rank(Task.new(no: 1, title: "a", priority: :Low))
  end
end
