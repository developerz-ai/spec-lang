// The `raylib` library's runtime for C++ (libs/raylib/real/cpp): a program's values as the shapes the calls take.
#include "lib/raylib/foreign.hpp"

namespace lib::raylib {

long long whole(const rt::Data& data) {
  const rt::Int n = data.integer();
  if (!n.fits_long()) rt::stop("E-RUNTIME", "an Int too large for a raylib call");
  return n.to_long();
}

Vec3 vec3(const rt::Data& data) {
  const auto& v = data.values();
  if (v.size() != 3) rt::stop("E-TYPE-MISMATCH", "a raylib vector is a record of three lengths");
  return {whole(v[0]), whole(v[1]), whole(v[2])};
}

Box box(const rt::Data& data) {
  const auto& v = data.values();
  if (v.size() != 2) rt::stop("E-TYPE-MISMATCH", "a raylib box is a record of two vectors");
  return {vec3(v[0]), vec3(v[1])};
}

Rgba color(const rt::Data& data) {
  const auto& v = data.values();
  if (v.size() != 3 && v.size() != 4) rt::stop("E-TYPE-MISMATCH", "a raylib color is a record of three or four bytes");
  const auto byte = [](const rt::Data& d) {
    const long long n = whole(d);
    if (n < 0 || n > 255) rt::stop("E-RUNTIME", "a color byte outside 0..=255");
    return static_cast<unsigned char>(n);
  };
  return {byte(v[0]), byte(v[1]), byte(v[2]), v.size() == 4 ? byte(v[3]) : static_cast<unsigned char>(255)};
}

std::size_t model_index(const rt::Data& data) {
  if (!data.is_int()) return data.variant();
  const long long n = whole(data);
  if (n < 0) rt::stop("E-RUNTIME", "a model index below 0");
  return static_cast<std::size_t>(n);
}

}  // namespace lib::raylib
