// The `raylib` library's runtime for C++ (libs/raylib/real/cpp): one adapter per `foreign c "raylib#Name"`.
// A program declares each call over its own types, so an adapter takes any type of the right shape:
//   a length is an Int of 1/256 of a raylib unit; a vector is a record of three lengths; a box is a record of
//   two vectors; a color is a record of three or four Ints from 0 to 255 (alpha 255 when absent); a model is
//   an Int, or a sum without data whose variants are the models in load order.
// The conversion to raylib's own structs, and every call into the C library, is in binding.cpp.
#pragma once

#include <cstddef>
#include <string>

#include "rt/data.hpp"

namespace lib::raylib {

// In 1/256 of a raylib unit.
struct Vec3 {
  long long x = 0;
  long long y = 0;
  long long z = 0;
};
struct Box {
  Vec3 lo;
  Vec3 hi;
};
struct Rgba {
  unsigned char r = 0;
  unsigned char g = 0;
  unsigned char b = 0;
  unsigned char a = 255;
};

// A program's value as the shape a call takes. Each stops E-TYPE-MISMATCH on a value of another shape.
long long whole(const rt::Data& data);
Vec3 vec3(const rt::Data& data);
Box box(const rt::Data& data);
Rgba color(const rt::Data& data);
std::size_t model_index(const rt::Data& data);

// The calls, over plain values.
namespace edge {
void clear_background(Rgba color);
// A perspective camera, y up, 45 degrees of vertical view.
void begin_mode_3d(Vec3 position, Vec3 target);
void end_mode_3d();
// The loaded shader until end_shader_mode; its one uniform is the time.
void begin_shader_mode(long long time);
void end_shader_mode();
void draw_cube(const Box& box, Rgba color);
void draw_cube_wires(const Box& box, Rgba color);
// A loaded model by its index in load order, turned about y by whole degrees.
void draw_model_ex(std::size_t model, Vec3 position, long long yaw);
void draw_text(const std::string& text, long long x, long long y, long long size, Rgba color);
bool is_key_down(long long key);
bool check_collision_boxes(const Box& a, const Box& b);
}  // namespace edge

template <class T>
rt::Data data_of(const T& value) {
  using rt::to_data;
  return to_data(value);
}

template <class C>
void ClearBackground(const C& color) {
  edge::clear_background(lib::raylib::color(data_of(color)));
}
template <class V>
void BeginMode3D(const V& position, const V& target) {
  edge::begin_mode_3d(vec3(data_of(position)), vec3(data_of(target)));
}
inline void EndMode3D() {
  edge::end_mode_3d();
}
inline void BeginShaderMode(rt::Int time) {
  edge::begin_shader_mode(whole(rt::Data{time}));
}
inline void EndShaderMode() {
  edge::end_shader_mode();
}
template <class B, class C>
void DrawCube(const B& box, const C& color) {
  edge::draw_cube(lib::raylib::box(data_of(box)), lib::raylib::color(data_of(color)));
}
template <class B, class C>
void DrawCubeWires(const B& box, const C& color) {
  edge::draw_cube_wires(lib::raylib::box(data_of(box)), lib::raylib::color(data_of(color)));
}
template <class M, class V>
void DrawModelEx(const M& model, const V& position, rt::Int yaw) {
  edge::draw_model_ex(model_index(data_of(model)), vec3(data_of(position)), whole(rt::Data{yaw}));
}
template <class C>
void DrawText(const rt::Text& text, rt::Int x, rt::Int y, rt::Int size, const C& color) {
  edge::draw_text(text, whole(rt::Data{x}), whole(rt::Data{y}), whole(rt::Data{size}),
                  lib::raylib::color(data_of(color)));
}
inline bool IsKeyDown(rt::Int key) {
  return edge::is_key_down(whole(rt::Data{key}));
}
template <class B>
bool CheckCollisionBoxes(const B& a, const B& b) {
  return edge::check_collision_boxes(box(data_of(a)), box(data_of(b)));
}

}  // namespace lib::raylib
