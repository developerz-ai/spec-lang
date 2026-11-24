// The `raylib` library's runtime for C++ (libs/raylib/real/cpp): its own test, through the real library.
// raylib's box test needs no window: it checks the conversion of lengths and boxes at the edge.
#include <iostream>

#include "lib/raylib/foreign.hpp"

namespace {

int failures = 0;

void expect(bool holds, const char* what) {
  if (!holds) {
    ++failures;
    std::cerr << "FAIL " << what << "\n";
  }
}

rt::Data vec(long long x, long long y, long long z) {
  return rt::Data::fields(0, {rt::Data{rt::Int{x}}, rt::Data{rt::Int{y}}, rt::Data{rt::Int{z}}});
}

lib::raylib::Box cube(long long at, long long size) {
  return lib::raylib::box(rt::Data::fields(0, {vec(at, at, at), vec(at + size, at + size, at + size)}));
}

}  // namespace

int main() {
  using lib::raylib::edge::check_collision_boxes;
  expect(check_collision_boxes(cube(0, 256), cube(128, 256)), "boxes that overlap collide");
  expect(check_collision_boxes(cube(0, 256), cube(256, 256)), "boxes that touch collide");
  expect(!check_collision_boxes(cube(0, 256), cube(257, 256)), "boxes one sub-unit apart do not");
  expect(!check_collision_boxes(cube(-1024, 256), cube(1024, 256)), "boxes far apart do not");

  const lib::raylib::Rgba three =
      lib::raylib::color(rt::Data::fields(0, {rt::Data{rt::Int{1}}, rt::Data{rt::Int{2}}, rt::Data{rt::Int{3}}}));
  expect(three.r == 1 && three.g == 2 && three.b == 3 && three.a == 255, "a color of three bytes is opaque");
  bool refused = false;
  try {
    lib::raylib::color(rt::Data::fields(0, {rt::Data{rt::Int{256}}, rt::Data{rt::Int{0}}, rt::Data{rt::Int{0}}}));
  } catch (const rt::Stop&) {
    refused = true;
  }
  expect(refused, "a color byte above 255 is refused, never wrapped");
  expect(lib::raylib::model_index(rt::Data::fields(2, {})) == 2 && lib::raylib::model_index(rt::Data{rt::Int{3}}) == 3,
         "a model is a variant's position or an Int");

  if (failures == 0) std::cout << "lib/raylib: every test passed\n";
  return failures == 0 ? 0 : 1;
}
