// Projected by hand from @cases of fn tag_text, fn line, and fn rank (src/todo/render.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "harness.hpp"
#include "support.hpp"
#include "todo/render/render.hpp"

using namespace todo::model;
using support::tags;
using support::task;

TEST(tag_text_cases) {
  CHECK_EQ(todo::render::tag_text({}), "");
  CHECK_EQ(todo::render::tag_text(tags({"work", "home"})), " #home #work");
}

TEST(line_cases) {
  CHECK_EQ(todo::render::line(task(1, "milk")), "1 [ ] milk");
  CHECK_EQ(todo::render::line(task(2, "ship", Status::Done, Priority::High, tags({"work"}))),
           "2 [x] ship !high #work");
}

TEST(rank_is_urgency_then_number) {
  CHECK(todo::render::rank(task(9, "a", Status::Open, Priority::High)) <
        todo::render::rank(task(1, "b", Status::Open, Priority::Normal)));
  CHECK(todo::render::rank(task(1, "a", Status::Open, Priority::Normal)) <
        todo::render::rank(task(2, "b", Status::Open, Priority::Normal)));
  CHECK(todo::render::rank(task(5, "a", Status::Open, Priority::Normal)) <
        todo::render::rank(task(1, "b", Status::Open, Priority::Low)));
}

TEST(badge_and_mark) {
  CHECK_EQ(todo::render::badge(Priority::Low), " !low");
  CHECK_EQ(todo::render::badge(Priority::Normal), "");
  CHECK_EQ(todo::render::mark(Status::Done), "x");
}
