// Projected by hand from Pad (src/hop/model.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

namespace hop::model {

// One frame of controls, sampled once per tick.
struct Pad {
  bool left = false;
  bool right = false;
  bool forward = false;
  bool back = false;
  bool jump = false;
  friend constexpr bool operator==(const Pad&, const Pad&) = default;
};

}  // namespace hop::model
