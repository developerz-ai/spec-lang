// Projected by hand from rule present (src/hop/screen.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include <variant>

#include "edge.hpp"
#include "hop/render/scene.hpp"
#include "hop/screen/screen.hpp"
#include "hop/world/view.hpp"

namespace hop::screen {
namespace {

struct Send {
  void operator()(const render::Cube& c) const { edge::draw_cube(c.box, c.color); }
  void operator()(const render::Wires& w) const { edge::draw_cube_wires(w.box, w.color); }
  void operator()(const render::Mesh& m) const { edge::draw_model(m.asset, m.at, m.yaw); }
};

}  // namespace

void present(const world::World& world) {
  const render::Frame f = render::scene(world::view(world));
  edge::clear_background(f.sky);
  edge::begin_mode_3d(f.eye, f.target);
  edge::begin_lit(f.time);
  for (const render::Draw& d : f.world) {
    std::visit(Send{}, d);
  }
  edge::end_lit();
  edge::end_mode_3d();
  for (const render::Label& l : f.labels) {
    edge::draw_text(l.text, l.x, l.y, l.size, l.color);
  }
}

}  // namespace hop::screen
