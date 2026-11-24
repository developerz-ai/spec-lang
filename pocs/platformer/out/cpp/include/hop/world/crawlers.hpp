// Projected by hand from state crawlers: Map[Int, Crawler] (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <algorithm>
#include <cstddef>
#include <initializer_list>
#include <span>
#include <vector>

#include "hop/model/level.hpp"

namespace hop::world {

// Map[Int, Crawler], iterated by ascending key. Every write in hop.world is `crawlers[c.no] = c`
// and a module owns its state, so the key is always the crawler's own number: the map is a vector
// of crawlers sorted by `no`, with no separate key.
class Crawlers {
 public:
  Crawlers() = default;
  Crawlers(std::initializer_list<model::Crawler> crawlers) {
    for (const model::Crawler& c : crawlers) {
      put(c);
    }
  }

  // crawlers[c.no] = c
  void put(const model::Crawler& c) {
    const auto at = find(c.no);
    if (at != by_no_.end() && at->no == c.no) {
      *at = c;
    } else {
      by_no_.insert(at, c);
    }
  }
  // crawlers.remove(no)
  void remove(geom::Int no) {
    const auto at = find(no);
    if (at != by_no_.end() && at->no == no) {
      by_no_.erase(at);
    }
  }
  bool contains(geom::Int no) const {
    return std::ranges::binary_search(by_no_, no, {}, &model::Crawler::no);
  }
  std::size_t size() const { return by_no_.size(); }
  void clear() { by_no_.clear(); }
  // crawlers.values: by crawler number.
  std::span<const model::Crawler> values() const { return by_no_; }
  // The same entries, writable in place, for a write that keeps every key (`no`) as it is.
  std::span<model::Crawler> values_in_place() { return by_no_; }
  friend bool operator==(const Crawlers&, const Crawlers&) = default;

 private:
  std::vector<model::Crawler>::iterator find(geom::Int no) {
    return std::ranges::lower_bound(by_no_, no, {}, &model::Crawler::no);
  }
  std::vector<model::Crawler> by_no_;
};

}  // namespace hop::world
