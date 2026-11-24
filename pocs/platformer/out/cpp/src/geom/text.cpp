// Projected by hand from vec_text and text interpolation of Int (src/hop/geom.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "hop/geom/text.hpp"

#include <charconv>

namespace hop::geom {

void append_int(std::string& out, Int n) {
  char digits[12];
  const auto end = std::to_chars(digits, digits + sizeof digits, n).ptr;
  out.append(digits, end);
}

void append_vec(std::string& out, Vec3 v) {
  append_int(out, v.x);
  out += ',';
  append_int(out, v.y);
  out += ',';
  append_int(out, v.z);
}

std::string vec_text(Vec3 v) {
  std::string out;
  append_vec(out, v);
  return out;
}

}  // namespace hop::geom
