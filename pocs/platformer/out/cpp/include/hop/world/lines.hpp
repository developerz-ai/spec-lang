// Projected by hand from the effect out.line (docs/game-vocabulary.md) as hop.world uses it (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <string>
#include <string_view>

#include "hop/geom/vec3.hpp"

namespace hop::world {

// Where a rule's out.line lines go, in the order sent, each followed by "\n". No I/O: the app
// prints the text. draw and shot print nothing, so they hand the rules a dropping sink and no
// line is ever formatted for them.
class Lines {
 public:
  static Lines kept() { return Lines(true); }
  static Lines dropped() { return Lines(false); }

  // "{text}"
  void line(std::string_view text);
  // "{tick} {what}"
  void event(geom::Int tick, std::string_view what);
  // "{tick} {what} {no}"
  void event(geom::Int tick, std::string_view what, geom::Int no);

  const std::string& text() const { return text_; }
  // Hands the lines over and leaves the sink empty.
  std::string take();

 private:
  explicit Lines(bool keep) : keep_(keep) {}
  bool keep_;
  std::string text_;
};

}  // namespace hop::world
