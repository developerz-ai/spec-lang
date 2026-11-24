// Projected by hand from vec_text (src/hop/geom.slg): how a failing test prints a Vec3. Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <optional>
#include <string>

#include "hop/geom/vec3.hpp"

namespace hop::geom {

// Found by argument-dependent lookup from harness::check_eq.
inline std::string show(const Vec3& v) { return vec_text(v); }

}  // namespace hop::geom
