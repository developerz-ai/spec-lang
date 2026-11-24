// Runs every check of a program as `spec-lang test` does and prints one JSON line per check:
// {"check": "<name>", "status": "pass" | "fail" | "blocked"}. A `@cases` row, a test (its `@given` world,
// then its steps), a property (generated arguments), a law (random traces).
#pragma once

#include <cstdint>
#include <functional>
#include <map>
#include <optional>
#include <string>
#include <variant>
#include <vector>

#include "rt/random.hpp"
#include "rt/value.hpp"

namespace rt::test {

// Counts and the base seed: the defaults of `spec-lang test`.
struct Config {
  std::uint64_t seed = 0;
  std::uint32_t runs = 100;
  std::uint32_t traces = 200;
  std::uint32_t steps = 40;
  std::uint32_t prefix = 10;
};

// Why a check did not pass, with the toolchain's diagnostic code.
struct Diag {
  std::string code;
  std::string message;
};

// The exception being handled, as a diagnostic. Call only inside a catch block.
Diag current_diag();

// `subject -> value;`
template <class F, class G>
void expect_value(F&& subject, G&& wanted) {
  std::optional<Failure> refused;
  using T = std::decay_t<std::invoke_result_t<G>>;
  std::optional<T> got;
  try {
    got.emplace(subject());
  } catch (const Failure& f) {
    refused = f;
  }
  const T want = wanted();
  if (refused) throw CheckFailed{"the step gave " + show(*refused) + ", expected " + str(want)};
  if (!(*got == want)) throw CheckFailed{"the step gave " + str(*got) + ", expected " + str(want)};
}

// `subject -> fail E;` or `subject -> fail E(data);`: with data, the data must be equal too.
template <class F>
void expect_fail(F&& subject, const Failure& wanted) {
  try {
    subject();
  } catch (const Failure& f) {
    if (f.name == wanted.name && (wanted.data.empty() || f.data == wanted.data)) return;
    throw CheckFailed{"the step gave " + show(f) + ", expected " + show(wanted)};
  }
  throw CheckFailed{"the step gave a value, expected " + show(wanted)};
}

template <class F>
void expect_fail(F&& subject, const char* name) {
  expect_fail(subject, Failure{name});
}
// The wanted failure's data is evaluated after the subject ran.
template <class F, class G>
  requires std::is_invocable_r_v<Failure, G>
void expect_fail(F&& subject, G&& wanted) {
  std::optional<Failure> got;
  try {
    subject();
  } catch (const Failure& f) {
    got = f;
  }
  const Failure want = wanted();
  expect_fail(
      [&] {
        if (got) throw *got;
      },
      want);
}

// A `@cases` row on a fn: its value equals the expected one.
template <class F, class G>
bool gives(F&& call, G&& wanted) {
  using T = std::decay_t<std::invoke_result_t<G>>;
  std::optional<T> got;
  try {
    got.emplace(call());
  } catch (const Failure&) {
  }
  const T want = wanted();
  return got.has_value() && *got == want;
}

// A `@cases` row that expects a refusal: compared by name only.
template <class F>
bool refuses(F&& call, const char* name) {
  try {
    call();
  } catch (const Failure& f) {
    return f.name == name;
  }
  return false;
}

void report(const std::string& check, const char* status, const std::optional<Diag>& diag);
// 1 when a check failed, else 0: what the checks' executable exits with.
int exit_code();

// How one run of a test ended.
template <class World>
struct Run {
  enum class Kind { Pass, Fail, Blocked } kind;
  World world = {};
  Diag diag = {};
};

// One generated value needs the generator, the pools of values already in a world, and a machine to check claims on.
template <class Machine, class Pools>
struct Gen {
  Rng& rng;
  const Pools& pools;
  Machine& m;
};

template <class Machine, class World, class Pools>
class Suite {
 public:
  using RunT = Run<World>;
  using GenT = Gen<Machine, Pools>;
  // A law with its parameters besides the trace drawn: true when it holds on the trace.
  using Bound = std::function<bool(Machine&, const Trace<World>&)>;

  struct RuleDriver {
    std::function<Event(GenT&)> draw;
    std::function<Reply(Machine&, const Event&)> call;
  };
  struct LawDef {
    std::string name;
    std::string decl;
    // Draws the law's other parameters, in order.
    std::function<Bound(GenT&)> bind;
    std::vector<int> traced;
    std::vector<int> start;
  };

  explicit Suite(Config config = {}) : config_(config) {}

  std::function<World(GenT&)> draw_world;
  std::map<int, RuleDriver> rules;
  std::vector<LawDef> laws;

  // ---- tests ----

  // The world a test starts from is built by `given`; `body` runs its steps.
  RunT run(const std::function<void(Suite&, Machine&)>& given, const std::function<void(Machine&)>& body) {
    std::optional<Machine> machine;
    try {
      machine.emplace();
      given(*this, *machine);
    } catch (const GivenEnd& end) {
      return end.run;
    } catch (...) {
      return RunT{RunT::Kind::Fail, {}, current_diag()};
    }
    try {
      body(*machine);
    } catch (const Blocked& b) {
      return RunT{RunT::Kind::Blocked, {}, Diag{b.code, b.message}};
    } catch (const Failure& f) {
      return RunT{RunT::Kind::Fail, {}, Diag{"F-TEST", "this step was refused: " + show(f)}};
    } catch (...) {
      return RunT{RunT::Kind::Fail, {}, current_diag()};
    }
    return RunT{RunT::Kind::Pass, std::move(machine->world), {}};
  }

  // `@given(test)`: the end world of another test, run once and remembered.
  World end_world(int test, const char* name, const std::function<RunT(Suite&)>& run_it) {
    auto found = ends_.find(test);
    if (found == ends_.end()) found = ends_.emplace(test, run_it(*this)).first;
    const RunT& r = found->second;
    if (r.kind == RunT::Kind::Pass) return r.world;
    if (r.kind == RunT::Kind::Blocked && r.diag.code == "B-FOREIGN") {
      throw GivenEnd{RunT{RunT::Kind::Blocked,
                          {},
                          Diag{"B-FOREIGN", std::string{"blocked by "} + name + ", which reached a foreign body"}}};
    }
    throw GivenEnd{RunT{RunT::Kind::Blocked, {}, Diag{"F-BLOCKED", std::string{"blocked by "} + name}}};
  }

  // A test without parameters, as a check.
  void test(const std::string& name, int id, const std::function<RunT(Suite&)>& run_it) {
    auto found = ends_.find(id);
    if (found == ends_.end()) found = ends_.emplace(id, run_it(*this)).first;
    settle(name, found->second, false);
  }

  // A `@cases` row on a fn: `row` gives whether the row holds.
  void case_fn(const std::string& name, const std::function<bool()>& row) {
    try {
      if (row()) {
        report(name, "pass", std::nullopt);
      } else {
        report(name, "fail", Diag{"F-CASE", "the case gave another result"});
      }
    } catch (...) {
      const Diag d = current_diag();
      report(name, d.code == "F-BLOCKED" || d.code == "B-FOREIGN" ? "blocked" : "fail", d);
    }
  }

  // A `@cases` row on a test: `given` builds its world, `row` evaluates the row's values there and runs the test.
  void case_test(const std::string& name, const std::function<void(Suite&, Machine&)>& given,
                 const std::function<RunT(Suite&, Machine&)>& row) {
    RunT r = run(given, [](Machine&) {});
    if (r.kind == RunT::Kind::Pass) {
      try {
        Machine m;
        m.world = std::move(r.world);
        r = row(*this, m);
      } catch (...) {
        r = RunT{RunT::Kind::Fail, {}, current_diag()};
      }
    }
    settle(name, r, true);
  }

  // A property: each run draws its arguments from its own seed; the lowest run that does not pass decides.
  void property(const std::string& name, const std::string& decl, const std::function<RunT(Suite&, GenT&)>& one) {
    for (std::uint32_t i = 0; i < config_.runs; ++i) {
      Rng rng{seed_for(config_.seed, decl, i)};
      const Pools pools{};
      RunT r{RunT::Kind::Pass, {}, {}};
      try {
        Machine m;
        GenT g{rng, pools, m};
        r = one(*this, g);
      } catch (...) {
        r = RunT{RunT::Kind::Fail, {}, current_diag()};
      }
      if (r.kind != RunT::Kind::Pass) {
        r.diag.message += " (run " + std::to_string(i) + ")";
        settle(name, r, false);
        return;
      }
    }
    report(name, "pass", std::nullopt);
  }

  // A law: every trace must hold; the lowest trace that does not decides.
  void law(std::size_t index) {
    const LawDef& def = laws[index];
    for (std::uint32_t i = 0; i < config_.traces; ++i) {
      std::optional<Diag> broken;
      try {
        Rng rng{seed_for(config_.seed, def.decl, i)};
        std::optional<World> start;
        if (i % 2 == 1) start = lawful_world(rng);
        const Walk walk = generate(rng, def.traced, def.start, std::move(start));
        const Bound bound = bind(def, walk.worlds, rng);
        broken = verdict(def, bound, walk.trace(walk.steps.size()));
        if (broken && broken->code == "F-LAW-FALSE") {
          broken = Diag{"F-LAW", "law " + def.name + " is false on trace " + std::to_string(i)};
        }
      } catch (...) {
        broken = current_diag();
      }
      if (broken) {
        report(def.name, broken->code == "B-FOREIGN" ? "blocked" : "fail", broken);
        return;
      }
    }
    report(def.name, "pass", std::nullopt);
  }

  // Thrown by a `@given` item that ends the test before its steps.
  struct GivenEnd {
    RunT run;
  };

 private:
  // One executed trace: its steps, and the start world followed by the world after each step.
  struct Walk {
    std::vector<std::pair<Event, Reply>> steps;
    std::vector<World> worlds;
    Trace<World> trace(std::size_t n) const {
      Trace<World> t;
      for (std::size_t i = 0; i < n && i < steps.size(); ++i) {
        t.steps.push_back(Step<World>{steps[i].first, steps[i].second, worlds[i], worlds[i + 1]});
      }
      for (std::size_t i = 0; i < worlds.size() && i < n + 1; ++i) t.states.push_back(worlds[i]);
      return t;
    }
  };

  void settle(const std::string& name, const RunT& r, bool as_case) {
    if (r.kind == RunT::Kind::Pass) {
      report(name, "pass", std::nullopt);
      return;
    }
    bool blocked = r.kind == RunT::Kind::Blocked || r.diag.code == "B-FOREIGN";
    if (as_case) blocked = r.diag.code == "F-BLOCKED" || r.diag.code == "B-FOREIGN";
    report(name, blocked ? "blocked" : "fail", r.diag);
  }

  // The law's other parameters, drawn with pools of every world given.
  Bound bind(const LawDef& def, const std::vector<World>& worlds, Rng& rng) {
    Pools pools{};
    for (const auto& w : worlds) pools.add(w);
    Machine m;
    GenT g{rng, pools, m};
    return def.bind(g);
  }

  // Nothing when the law holds; F-LAW-FALSE when it is false; else what broke it.
  std::optional<Diag> verdict(const LawDef& def, const Bound& bound, const Trace<World>& trace) {
    try {
      Machine m;
      if (bound(m, trace)) return std::nullopt;
      return Diag{"F-LAW-FALSE", ""};
    } catch (const Failure& f) {
      return Diag{"F-LAW", "the law " + def.name + " failed with " + f.name + " instead of giving a Bool"};
    } catch (...) {
      return current_diag();
    }
  }

  // A world drawn from the states' declared types on which every law holds at the start, if 200 tries find one.
  std::optional<World> lawful_world(Rng& rng) {
    Machine m;
    for (int tries = 0; tries < 200; ++tries) {
      const Pools none{};
      GenT g{rng, none, m};
      World w = draw_world(g);
      const Walk walk{{}, {w}};
      bool lawful = true;
      for (const LawDef& def : laws) {
        const Bound bound = bind(def, walk.worlds, rng);
        if (verdict(def, bound, walk.trace(0))) {
          lawful = false;
          break;
        }
      }
      if (lawful) return w;
    }
    return std::nullopt;
  }

  Event random_event(Machine& m, Rng& rng, const std::vector<int>& from) {
    if (from.empty()) stop("E-GEN", "a trace with no rules");
    const int rule = from[rng.below(from.size())];
    Pools pools{};
    pools.add(m.world);
    GenT g{rng, pools, m};
    return rules.at(rule).draw(g);
  }

  Walk generate(Rng& rng, const std::vector<int>& traced, const std::vector<int>& start_rules,
                std::optional<World> start) {
    Machine m;
    std::uint64_t prefix = 0;
    if (start) {
      m.world = std::move(*start);
    } else {
      prefix = rng.below(std::uint64_t{config_.prefix} + 1);
    }
    for (; prefix > 0; --prefix) {
      const Event e = random_event(m, rng, start_rules);
      rules.at(e.rule).call(m, e);
    }
    Walk walk{{}, {m.world}};
    for (std::uint32_t i = 0; i < config_.steps; ++i) {
      Event e = random_event(m, rng, traced);
      Reply r = rules.at(e.rule).call(m, e);
      walk.steps.emplace_back(std::move(e), std::move(r));
      walk.worlds.push_back(m.world);
    }
    return walk;
  }

  Config config_;
  std::map<int, RunT> ends_;
};

}  // namespace rt::test
