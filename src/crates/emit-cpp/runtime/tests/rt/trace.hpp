// Traces for laws (docs/idea/01-language-card.md, Traces): a seeded sequence of events from a start world,
// with each step's world before and after, its event, and whether it was refused.
#pragma once

#include <cstdint>
#include <vector>

#include "rt/random.hpp"

namespace rt::test {

template <class World, class Event>
struct Step {
  World before;
  World after;
  Event event;
  bool refused = false;
};

template <class World, class Event>
struct Trace {
  World start;
  std::vector<Step<World, Event>> steps;

  // t.states: the start world, then the world after each step.
  [[nodiscard]] std::vector<World> states() const {
    std::vector<World> out{start};
    for (const auto& s : steps) {
      out.push_back(s.after);
    }
    return out;
  }
};

// `steps` events drawn by `draw(prng)`, each run by `apply(event, world)` (true when refused).
template <class World, class Event, class Draw, class Apply>
Trace<World, Event> trace(std::uint64_t seed, World start, int steps, Draw draw, Apply apply) {
  random::Prng prng(seed);
  Trace<World, Event> t{start, {}};
  World world = std::move(start);
  for (int i = 0; i < steps; ++i) {
    Event event = draw(prng);
    Step<World, Event> s{world, world, event, false};
    s.refused = apply(std::move(event), world);
    s.after = world;
    t.steps.push_back(std::move(s));
  }
  return t;
}

}  // namespace rt::test
