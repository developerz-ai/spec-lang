// Projected by hand from every test in todo.tests.scenarios (src/todo/tests/scenarios.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include <cstdint>
#include <limits>
#include <optional>
#include <string>
#include <utility>

#include "harness.hpp"
#include "support.hpp"
#include "todo/board/rules.hpp"

using namespace todo::model;
using support::no;
using support::tags;
using support::task;
using support::title;
using todo::board::RuleError;
using todo::board::World;

namespace {

// The out handler: everything out.line sent in this test. A refused call sends nothing.
struct Board {
  World world;
  std::string out;

  void take(std::vector<std::string> lines) {
    for (std::string& l : lines) {
      out += l;
      out += '\n';
    }
  }
  template <class R>
  R commit(todo::board::Outcome<R> o) {
    world = std::move(o.world);
    take(std::move(o.lines));
    return o.reply;
  }
  template <class R>
  std::optional<RuleError> commit(todo::board::Result<R> r) {
    if (!r) {
      return r.error();
    }
    commit(std::move(*r));
    return std::nullopt;
  }
};

World three_open_one_done_world() {
  World w;
  w.tasks.emplace(no(1), task(1, "low one", Status::Open, Priority::Low));
  w.tasks.emplace(no(2), task(2, "plain"));
  w.tasks.emplace(no(3), task(3, "urgent", Status::Open, Priority::High, tags({"work"})));
  w.tasks.emplace(no(4), task(4, "finished", Status::Done, Priority::Normal, tags({"work", "home"})));
  w.next_no = no(5);
  return w;
}

Board given_three_open_one_done() { return Board{three_open_one_done_world(), {}}; }

}  // namespace

TEST(three_open_one_done) {
  const Board b = given_three_open_one_done();
  CHECK_EQ(b.world.tasks.size(), 4U);
  CHECK_EQ(b.world.next_no.value(), 5U);
  CHECK_EQ(b.out, "");
}

TEST(numbers_start_at_one) {
  Board b;
  CHECK_EQ(b.commit(todo::board::add(b.world, title("milk"))).value(), 1U);
  CHECK_EQ(
      b.commit(todo::board::add(b.world, title("bread"), Priority::Normal, tags({"shop", "home"}))).value(),
      2U);
  CHECK_EQ(b.out, "added 1\nadded 2\n");
}

TEST(numbers_are_never_reused) {
  Board b;
  CHECK_EQ(b.commit(todo::board::add(b.world, title("a"))).value(), 1U);
  CHECK_EQ(b.commit(todo::board::add(b.world, title("b"))).value(), 2U);
  CHECK(!b.commit(todo::board::remove(b.world, no(2))).has_value());
  CHECK_EQ(b.commit(todo::board::add(b.world, title("c"))).value(), 3U);
}

TEST(empty_board_says_so) {
  Board b;
  b.take(todo::board::list(b.world));
  b.take(todo::board::stats(b.world));
  CHECK_EQ(b.out, "nothing to do\n0 open, 0 done\n");
}

TEST(list_ranks_urgent_first) {
  Board b = given_three_open_one_done();
  b.take(todo::board::list(b.world));
  CHECK_EQ(b.out, "3 [ ] urgent !high #work\n2 [ ] plain\n1 [ ] low one !low\n");
}

TEST(list_all_and_by_tag) {
  Board b = given_three_open_one_done();
  b.take(todo::board::list(b.world, true, support::tag("work")));
  CHECK_EQ(b.out, "3 [ ] urgent !high #work\n4 [x] finished #home #work\n");
}

TEST(done_and_reopen_move_once) {
  Board b = given_three_open_one_done();
  CHECK(!b.commit(todo::board::done(b.world, no(2))).has_value());
  CHECK(b.commit(todo::board::done(b.world, no(2))) == RuleError::AlreadyDone);
  CHECK(!b.commit(todo::board::reopen(b.world, no(2))).has_value());
  CHECK(b.commit(todo::board::reopen(b.world, no(2))) == RuleError::NotDone);
  CHECK_EQ(b.out, "2 [x] plain\n2 [ ] plain\n");
}

TEST(rename_keeps_the_rest) {
  Board b = given_three_open_one_done();
  CHECK(!b.commit(todo::board::rename(b.world, no(3), title("urgent, really"))).has_value());
  CHECK(b.world.tasks.at(no(3)) == task(3, "urgent, really", Status::Open, Priority::High, tags({"work"})));
}

TEST(clear_removes_only_done) {
  Board b = given_three_open_one_done();
  CHECK_EQ(b.commit(todo::board::clear(b.world)), 1U);
  b.take(todo::board::stats(b.world));
  CHECK_EQ(b.out, "cleared 1\n3 open, 0 done\n");
  CHECK(!b.world.tasks.contains(no(4)));
}

// A property: no is generated. Small numbers, the boundary, and the far end of the type.
TEST(unknown_numbers_are_refused) {
  std::vector<std::uint64_t> samples;
  for (std::uint64_t n = 1; n <= 200; ++n) {
    samples.push_back(n);
  }
  for (const std::uint64_t n :
       {std::uint64_t{1} << 53U, (std::uint64_t{1} << 53U) + 1, std::numeric_limits<std::uint64_t>::max()}) {
    samples.push_back(n);
  }
  for (const std::uint64_t n : samples) {
    Board b = given_three_open_one_done();
    if (n <= 4) {
      continue;
    }
    CHECK(b.commit(todo::board::done(b.world, no(n))) == RuleError::NoTask);
    CHECK(b.commit(todo::board::reopen(b.world, no(n))) == RuleError::NoTask);
    CHECK(b.commit(todo::board::rename(b.world, no(n), title("x"))) == RuleError::NoTask);
    CHECK(b.commit(todo::board::remove(b.world, no(n))) == RuleError::NoTask);
    CHECK_EQ(b.out, "");
  }
}
