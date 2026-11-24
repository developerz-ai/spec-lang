// Projected by hand from the frame hash of shot (docs/contract.md section 2): 16 lower-case hex digits. Stand-in for spec-lang build output; change the spec, not this file.
#include "game/app/fnv.hpp"

namespace game::app {

std::string hex16(std::uint64_t value) {
  std::string out(16, '0');
  for (int i = 15; i >= 0; --i) {
    out[static_cast<std::size_t>(i)] = "0123456789abcdef"[value & 0xF];
    value >>= 4;
  }
  return out;
}

}  // namespace game::app
