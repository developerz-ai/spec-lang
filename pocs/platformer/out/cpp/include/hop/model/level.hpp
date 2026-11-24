// Projected by hand from Solid, Mover, Gem, Crawler, Level (src/hop/model.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <algorithm>
#include <span>
#include <string_view>

#include "hop/geom/box.hpp"
#include "hop/model/rgb.hpp"

namespace hop::model {

using geom::Int;

struct Solid {
  geom::Box box;
  Rgb color;
  friend constexpr bool operator==(const Solid&, const Solid&) = default;
};

// A one-way platform: it carries whoever lands on it from above and never blocks from the side or below.
struct Mover {
  Int no;
  geom::Box home;
  geom::Vec3 step;
  Int span;  // 1..=1024
  Rgb color;
  friend constexpr bool operator==(const Mover&, const Mover&) = default;
};

struct Gem {
  Int no;
  geom::Vec3 at;
  friend constexpr bool operator==(const Gem&, const Gem&) = default;
};

// A crawler patrols along x between lo and hi.
struct Crawler {
  Int no;
  geom::Vec3 at;
  Int lo;
  Int hi;
  Int dir;    // -1..=1
  Int speed;  // 1..=8
  friend constexpr bool operator==(const Crawler&, const Crawler&) = default;
};

// A level is a constant (values are immutable), so its lists are views of data with static storage:
// copying a Level copies no list. Whoever builds one keeps the arrays alive.
struct Level {
  std::string_view name;
  geom::Vec3 start;
  std::span<const Solid> solids;
  std::span<const Mover> movers;
  std::span<const Gem> gems;
  std::span<const Crawler> crawlers;
  geom::Vec3 goal;

  // == is structural on every type: the lists compare by content, not by address.
  friend constexpr bool operator==(const Level& a, const Level& b) {
    return a.name == b.name && a.start == b.start && std::ranges::equal(a.solids, b.solids) &&
           std::ranges::equal(a.movers, b.movers) && std::ranges::equal(a.gems, b.gems) &&
           std::ranges::equal(a.crawlers, b.crawlers) && a.goal == b.goal;
  }
};

}  // namespace hop::model
