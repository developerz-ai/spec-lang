// Projected by hand from the effect out.line (docs/game-vocabulary.md) as hop.world uses it (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "hop/world/lines.hpp"

#include <utility>

#include "hop/geom/text.hpp"

namespace hop::world {

void Lines::line(std::string_view text) {
  if (keep_) {
    text_ += text;
    text_ += '\n';
  }
}

void Lines::event(geom::Int tick, std::string_view what) {
  if (keep_) {
    geom::append_int(text_, tick);
    text_ += ' ';
    text_ += what;
    text_ += '\n';
  }
}

void Lines::event(geom::Int tick, std::string_view what, geom::Int no) {
  if (keep_) {
    geom::append_int(text_, tick);
    text_ += ' ';
    text_ += what;
    text_ += ' ';
    geom::append_int(text_, no);
    text_ += '\n';
  }
}

std::string Lines::take() { return std::exchange(text_, {}); }

}  // namespace hop::world
