// Projected by hand from state taken: Set[Int] (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <algorithm>
#include <cstddef>
#include <initializer_list>
#include <vector>

#include "hop/geom/vec3.hpp"

namespace hop::world {

// Set[Int], iterated by ascending key: a sorted vector, since a level has a handful of gems.
class Taken {
 public:
  Taken() = default;
  Taken(std::initializer_list<geom::Int> numbers) {
    for (const geom::Int n : numbers) {
      insert(n);
    }
  }

  bool contains(geom::Int n) const { return std::ranges::binary_search(sorted_, n); }
  void insert(geom::Int n) {
    const auto at = std::ranges::lower_bound(sorted_, n);
    if (at == sorted_.end() || *at != n) {
      sorted_.insert(at, n);
    }
  }
  std::size_t size() const { return sorted_.size(); }
  auto begin() const { return sorted_.begin(); }
  auto end() const { return sorted_.end(); }
  friend bool operator==(const Taken&, const Taken&) = default;

 private:
  std::vector<geom::Int> sorted_;
};

}  // namespace hop::world
