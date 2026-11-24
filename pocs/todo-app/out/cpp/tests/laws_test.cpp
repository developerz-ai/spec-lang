// Projected by hand from every law in todo.laws (src/todo/laws.slg): seeded traces, every law after every step. Stand-in for spec-lang build output; change the spec, not this file.
#include <array>
#include <cstdint>
#include <optional>
#include <set>
#include <string>
#include <vector>

#include "harness.hpp"
#include "support.hpp"
#include "todo/board/rules.hpp"

using namespace todo::model;
using todo::board::World;

namespace {

constexpr int traces = 400;
constexpr int steps_per_trace = 40;

// splitmix64: deterministic, so a failing seed replays exactly.
class Prng {
 public:
  explicit Prng(std::uint64_t seed) : state_(seed) {}
  std::uint64_t next() {
    std::uint64_t z = (state_ += 0x9E3779B97F4A7C15ULL);
    z = (z ^ (z >> 30U)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27U)) * 0x94D049BB133111EBULL;
    return z ^ (z >> 31U);
  }
  std::uint64_t below(std::uint64_t n) { return next() % n; }

 private:
  std::uint64_t state_;
};

enum class Kind { Add, Done, Reopen, Rename, Remove, Clear, List, Stats };

struct Step {
  Kind kind;
  std::optional<std::uint64_t> no;  // event.no for the rules that name a task
  bool refused = false;
  World before;
  World after;
};

constexpr std::array<const char*, 4> titles{"milk", "  café ", "ship it", "x"};
constexpr std::array<const char*, 4> tag_pool{"home", "work", "a", "deep-work"};

std::set<Tag> random_tags(Prng& r) {
  std::set<Tag> out;
  const auto n = r.below(3);
  for (std::uint64_t i = 0; i < n; ++i) {
    out.insert(support::tag(tag_pool[r.below(tag_pool.size())]));
  }
  return out;
}

template <class R>
void settle(Step& s, todo::board::Result<R> r) {
  s.refused = !r.has_value();
  s.after = r ? std::move(r->world) : s.before;
}

Step random_step(const World& w, Prng& r) {
  Step s{
      .kind = static_cast<Kind>(r.below(8)), .no = std::nullopt, .refused = false, .before = w, .after = w};
  // Numbers near the live range: present, removed and never issued alike.
  const TaskNo no = support::no(1 + r.below(w.next_no.value() + 2));
  switch (s.kind) {
    case Kind::Add: {
      static constexpr std::array<Priority, 3> ps{Priority::Low, Priority::Normal, Priority::High};
      s.after =
          todo::board::add(w, support::title(titles[r.below(titles.size())]), ps[r.below(3)], random_tags(r))
              .world;
      break;
    }
    case Kind::Done:
      s.no = no.value();
      settle(s, todo::board::done(w, no));
      break;
    case Kind::Reopen:
      s.no = no.value();
      settle(s, todo::board::reopen(w, no));
      break;
    case Kind::Rename:
      s.no = no.value();
      settle(s, todo::board::rename(w, no, support::title(titles[r.below(titles.size())])));
      break;
    case Kind::Remove:
      s.no = no.value();
      settle(s, todo::board::remove(w, no));
      break;
    case Kind::Clear:
      s.after = todo::board::clear(w).world;
      break;
    case Kind::List: {
      const auto lines = todo::board::list(w, r.below(2) == 1);
      harness::check(!lines.empty(), "list always says something");
      break;  // a read returns no world: after is before
    }
    case Kind::Stats:
      harness::check(todo::board::stats(w).size() == 1, "stats is one line");
      break;
  }
  return s;
}

bool is_change(Kind k) { return k != Kind::List && k != Kind::Stats; }

// law numbers_below_next, over every state.
void numbers_below_next(const World& w) {
  for (const auto& [no, t] : w.tasks) {
    harness::check(no < w.next_no, "numbers_below_next");
    harness::check(t.no == no, "a task is filed under its own number");
  }
}

void check_laws(const Step& s) {
  numbers_below_next(s.after);
  if (is_change(s.kind)) {
    harness::check(s.after.next_no >= s.before.next_no, "next_only_grows");
    if (s.refused) {
      harness::check(s.after == s.before, "refusal_changes_nothing");
    }
  }
  if (s.no.has_value()) {  // done | reopen | rename | remove
    std::set<TaskNo> ks;
    for (const auto& [k, t] : s.before.tasks) {
      ks.insert(k);
    }
    for (const auto& [k, t] : s.after.tasks) {
      ks.insert(k);
    }
    ks.insert(s.before.next_no);
    for (const TaskNo k : ks) {
      if (k.value() == *s.no) {
        continue;
      }
      const auto b = s.before.tasks.find(k);
      const auto a = s.after.tasks.find(k);
      const bool same = (b == s.before.tasks.end()) ? a == s.after.tasks.end()
                                                    : (a != s.after.tasks.end() && a->second == b->second);
      harness::check(same, "others_untouched");
    }
  }
  if (s.kind == Kind::Clear) {
    std::size_t open_before = 0;
    for (const auto& [k, t] : s.before.tasks) {
      open_before += t.status == Status::Open ? 1 : 0;
    }
    for (const auto& [k, t] : s.after.tasks) {
      harness::check(t.status == Status::Open, "clear_keeps_every_open_task: all open");
    }
    harness::check(s.after.tasks.size() == open_before, "clear_keeps_every_open_task: none lost");
  }
  if (!is_change(s.kind)) {
    harness::check(s.after == s.before, "reads_change_nothing");
  }
}

}  // namespace

TEST(every_law_after_every_step) {
  std::array<int, 8> kinds_seen{};
  int refusals = 0;
  for (std::uint64_t seed = 1; seed <= traces; ++seed) {
    Prng r(seed);
    World w;
    numbers_below_next(w);
    for (int i = 0; i < steps_per_trace; ++i) {
      const Step s = random_step(w, r);
      check_laws(s);
      ++kinds_seen[static_cast<std::size_t>(s.kind)];
      refusals += s.refused ? 1 : 0;
      w = s.after;
    }
  }
  for (const int n : kinds_seen) {
    CHECK(n > 0);  // every command was exercised
  }
  CHECK(refusals > 0);  // and refusals happened, so refusal_changes_nothing was tested
}
