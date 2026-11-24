// Projected by hand from mode_text, summary (src/hop/world.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include <string_view>

#include "hop/geom/text.hpp"
#include "hop/world/rules.hpp"

namespace hop::world {
namespace {

constexpr std::string_view mode_text(model::Mode m) {
  switch (m) {
    case model::Mode::Playing:
      return "playing";
    case model::Mode::Won:
      return "won";
    case model::Mode::Lost:
      return "lost";
  }
  return "";
}

}  // namespace

// "end {ticks} {mode} gems {taken.size}/{stage.gems.size} lives {lives} at {x},{y},{z}"
std::string summary(const World& world) {
  std::string out = "end ";
  geom::append_int(out, world.ticks);
  out += ' ';
  out += mode_text(world.mode);
  out += " gems ";
  geom::append_int(out, static_cast<Int>(world.taken.size()));
  out += '/';
  geom::append_int(out, static_cast<Int>(world.stage.gems.size()));
  out += " lives ";
  geom::append_int(out, world.lives);
  out += " at ";
  geom::append_vec(out, world.player.at);
  return out;
}

}  // namespace hop::world
