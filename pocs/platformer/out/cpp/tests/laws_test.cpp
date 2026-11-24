// Projected by hand from the laws never_inside_a_solid, speed_is_bounded, taken_only_grows, lives_never_rise, lost_means_no_lives, over_is_final, one_tick_per_step, taken_are_real_gems (src/hop/laws.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include <algorithm>
#include <cstdio>
#include <cstdlib>
#include <optional>
#include <span>
#include <string>
#include <vector>

#include "harness.hpp"
#include "hop/physics/bodies.hpp"
#include "hop/world/rules.hpp"
#include "support/random.hpp"
#include "support/scripts.hpp"

using hop::geom::Int;
using hop::model::Mode;
using hop::model::Pad;
using hop::world::World;

namespace {

// type Play = world.start | world.step
struct Event {
  std::optional<hop::levels::LevelNo> start;  // set: world.start(level); else world.step(pad)
  Pad pad;
};

// One element of t.steps: event, reply, before, after.
struct Step {
  Event event;
  bool ok;
  World before;
  World after;
};

constexpr int trace_count = 400;
constexpr int trace_length = 400;

Pad random_pad(support::Random& random) {
  const std::uint64_t bits = random.next();
  return {.left = (bits & 1) != 0,
          .right = (bits & 2) != 0,
          .forward = (bits & 4) != 0,
          .back = (bits & 8) != 0,
          .jump = (bits & 16) != 0};
}

// The pads of a winning script, one per tick.
std::vector<Pad> pads_of(std::span<const scripts::Hold> holds) {
  std::vector<Pad> pads;
  for (const scripts::Hold& h : holds) {
    pads.insert(pads.end(), static_cast<std::size_t>(h.ticks), scripts::pad_of(h.keys));
  }
  return pads;
}

// A trace from the declared world: start of a random level, then pads. Five kinds of trace, by
// seed, so the laws see more than a hero shuffling at the start:
//   0  a new random pad every tick
//   1  random pads, each held for 1 to 40 ticks
//   2  always right, jump held at random: runs into gaps, walls and crawlers, and loses
//   3  a winning script with a few ticks replaced: reaches gems, movers and the flag
//   4  a winning script up to a random tick, then as kind 2: lands on and beside the crawlers
// Any trace may start a level again in the middle, and every trace keeps stepping after the game
// is over, which is what over_is_final is about.
std::vector<Step> trace(std::uint64_t seed) {
  support::Random random(seed * 0x2545F4914F6CDD1DULL + 1);
  const int kind = static_cast<int>(seed % 5);
  const auto random_level = [&] { return *hop::levels::LevelNo::make(random.between(1, 2)); };

  hop::levels::LevelNo level = random_level();
  std::vector<Pad> scripted;
  if (kind >= 3) {
    scripted = level.value() == 1 ? pads_of(scripts::meadow_win) : pads_of(scripts::ridge_win);
  }
  const int length = std::max(trace_length, static_cast<int>(scripted.size()) + 20);
  const std::uint32_t noise = 50U << (seed / 5 % 8);  // one tick in 50, 100, ... 6400 is replaced
  if (kind == 4) {
    scripted.resize(static_cast<std::size_t>(random.between(0, static_cast<Int>(scripted.size()))));
  }
  const std::uint32_t restart = kind >= 3 ? 4096 : 256;  // one event in this many starts a level again

  std::vector<Step> steps;
  steps.reserve(static_cast<std::size_t>(length) + 1);
  World world;
  hop::world::Lines lines = hop::world::Lines::dropped();
  Pad held;
  int hold_left = 0;
  std::size_t tick = 0;
  for (int i = 0; i <= length; ++i) {
    Event event;
    if (i == 0 || random.one_in(restart)) {
      event.start = i == 0 ? level : random_level();
      if (i > 0) {
        scripted.clear();  // the script was for the level this trace began with
      }
    } else if (kind == 0) {
      event.pad = random_pad(random);
    } else if (kind == 1) {
      if (hold_left == 0) {
        held = random_pad(random);
        hold_left = random.between(1, 40);
      }
      --hold_left;
      event.pad = held;
    } else if (kind == 2 || (kind == 4 && tick >= scripted.size())) {
      if (hold_left == 0) {
        held = {.right = true, .jump = random.one_in(2)};
        hold_left = random.between(1, 40);
      }
      --hold_left;
      event.pad = held;
    } else if (kind == 4) {
      event.pad = scripted[tick++];
    } else {
      event.pad = tick < scripted.size() && !random.one_in(noise) ? scripted[tick] : random_pad(random);
      ++tick;
    }
    Step step{event, true, world, world};
    if (event.start) {
      hop::world::start(step.after, *event.start, lines);
    } else {
      step.ok = hop::world::step(step.after, event.pad, lines).has_value();
    }
    world = step.after;
    steps.push_back(std::move(step));
  }
  return steps;
}

// Runs a claim over every state of every trace (t.states: the first world and each one after).
template <class Claim>
void all_states(Claim claim) {
  for (int seed = 0; seed < trace_count; ++seed) {
    const std::vector<Step> steps = trace(static_cast<std::uint64_t>(seed));
    if (!claim(steps.front().before)) {
      harness::fail("seed " + std::to_string(seed) + ": fails in the first world");
    }
    for (std::size_t i = 0; i < steps.size(); ++i) {
      if (!claim(steps[i].after)) {
        harness::fail("seed " + std::to_string(seed) + ": fails after event " + std::to_string(i));
      }
    }
  }
}

// Runs a claim over every world.step of every trace (Trace[world.step]: starts are not steps).
template <class Claim>
void all_steps(Claim claim) {
  for (int seed = 0; seed < trace_count; ++seed) {
    const std::vector<Step> steps = trace(static_cast<std::uint64_t>(seed));
    for (std::size_t i = 0; i < steps.size(); ++i) {
      if (!steps[i].event.start && !claim(steps[i])) {
        harness::fail("seed " + std::to_string(seed) + ": fails at event " + std::to_string(i));
      }
    }
  }
}

}  // namespace

// The hero is never inside a solid.
TEST(never_inside_a_solid) {
  all_states([](const World& w) {
    return std::ranges::none_of(w.stage.solids, [&](const hop::model::Solid& s) {
      return hop::geom::overlaps(hop::physics::body(w.player.at), s.box);
    });
  });
}

// Speeds stay in bounds, so a body never passes through a solid in one tick.
TEST(speed_is_bounded) {
  all_states([](const World& w) {
    const hop::geom::Vec3 v = w.player.vel;
    return std::abs(v.x) <= hop::physics::run_speed && std::abs(v.z) <= hop::physics::run_speed &&
           -hop::physics::max_fall <= v.y && v.y <= hop::physics::jump_speed;
  });
}

// A gem once taken stays taken until the level starts again.
TEST(taken_only_grows) {
  all_steps([](const Step& s) {
    return std::ranges::all_of(s.before.taken, [&](Int g) { return s.after.taken.contains(g); });
  });
}

// Lives only go down during play.
TEST(lives_never_rise) {
  all_steps([](const Step& s) { return s.after.lives <= s.before.lives; });
}

// No lives left is exactly a lost game.
TEST(lost_means_no_lives) {
  all_states([](const World& w) { return (w.lives == 0) == (w.mode == Mode::Lost); });
}

// A finished game is final: a step is refused and nothing moves.
TEST(over_is_final) {
  all_steps([](const Step& s) { return s.before.mode == Mode::Playing || (!s.ok && s.after == s.before); });
}

// Time moves one tick per accepted step.
TEST(one_tick_per_step) {
  all_steps([](const Step& s) { return !s.ok || s.after.ticks == s.before.ticks + 1; });
}

// A taken gem is one the level has.
TEST(taken_are_real_gems) {
  all_states([](const World& w) {
    return std::ranges::all_of(w.taken, [&](Int n) {
      return std::ranges::any_of(w.stage.gems, [&](const hop::model::Gem& g) { return g.no == n; });
    });
  });
}

// Not a law: what the traces reached, so a law that held is known not to have held on nothing.
TEST(the_traces_reach_every_ending) {
  int won = 0;
  int lost = 0;
  int refused = 0;
  int gems = 0;
  int stomps = 0;
  int rides = 0;
  long events = 0;
  for (int seed = 0; seed < trace_count; ++seed) {
    for (const Step& s : trace(static_cast<std::uint64_t>(seed))) {
      ++events;
      if (s.event.start) {
        continue;
      }
      won += s.before.mode == Mode::Playing && s.after.mode == Mode::Won;
      lost += s.before.mode == Mode::Playing && s.after.mode == Mode::Lost;
      refused += !s.ok;
      gems += s.after.taken.size() > s.before.taken.size();
      stomps += s.ok && s.after.crawlers.size() < s.before.crawlers.size();
      rides += s.after.player.riding.has_value() && !s.before.player.riding.has_value();
    }
  }
  std::printf("  %d traces, %ld events: %d won, %d lost, %d refused steps, %d gems, %d stomps, %d rides\n",
              trace_count, events, won, lost, refused, gems, stomps, rides);
  CHECK(trace_count >= 200 && trace_length >= 200);
  CHECK(won > 0);
  CHECK(lost > 0);
  CHECK(refused > 0);
  CHECK(gems > 0);
  CHECK(stomps > 0);
  CHECK(rides > 0);
}
