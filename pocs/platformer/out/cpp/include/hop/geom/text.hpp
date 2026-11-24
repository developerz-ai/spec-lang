// Projected by hand from text interpolation "{n}" of Int (docs/idea/01-language-card.md, Expressions) as used by vec_text (src/hop/geom.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <string>

#include "hop/geom/vec3.hpp"

namespace hop::geom {

// Appends the decimal digits of n, with a leading '-' when negative.
void append_int(std::string& out, Int n);

// Appends "x,y,z".
void append_vec(std::string& out, Vec3 v);

}  // namespace hop::geom
