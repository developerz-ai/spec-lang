// Projected by hand from rgb_text, draw_text, frame_lines, dump (src/hop/render.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "hop/render/text.hpp"

#include "hop/geom/text.hpp"
#include "hop/render/scene.hpp"
#include "hop/world/view.hpp"

namespace hop::render {
namespace {

void append_rgb(std::string& out, model::Rgb c) {
  geom::append_int(out, c.r);
  out += ',';
  geom::append_int(out, c.g);
  out += ',';
  geom::append_int(out, c.b);
}

std::string box_text(std::string_view word, const geom::Box& b, model::Rgb color) {
  std::string out(word);
  geom::append_vec(out, b.lo);
  out += ' ';
  geom::append_vec(out, b.hi);
  out += ' ';
  append_rgb(out, color);
  return out;
}

struct DrawText {
  std::string operator()(const Cube& c) const { return box_text("cube ", c.box, c.color); }
  std::string operator()(const Wires& w) const { return box_text("wires ", w.box, w.color); }
  std::string operator()(const Mesh& m) const {
    std::string out = "mesh ";
    out += asset_name(m.asset);
    out += ' ';
    geom::append_vec(out, m.at);
    out += ' ';
    geom::append_int(out, m.yaw);
    return out;
  }
};

}  // namespace

std::string draw_text(const Draw& d) { return std::visit(DrawText{}, d); }

std::vector<std::string> frame_lines(const Frame& f) {
  std::vector<std::string> lines;
  lines.reserve(3 + f.world.size() + f.labels.size());

  std::string sky = "sky ";
  append_rgb(sky, f.sky);
  lines.push_back(std::move(sky));

  std::string camera = "camera ";
  geom::append_vec(camera, f.eye);
  camera += ' ';
  geom::append_vec(camera, f.target);
  lines.push_back(std::move(camera));

  std::string time = "time ";
  geom::append_int(time, f.time);
  lines.push_back(std::move(time));

  for (const Draw& d : f.world) {
    lines.push_back(draw_text(d));
  }
  for (const Label& l : f.labels) {
    std::string label = "label ";
    geom::append_int(label, l.x);
    label += ' ';
    geom::append_int(label, l.y);
    label += ' ';
    geom::append_int(label, l.size);
    label += ' ';
    append_rgb(label, l.color);
    label += ' ';
    label += l.text;
    lines.push_back(std::move(label));
  }
  return lines;
}

std::vector<std::string> dump(const world::World& world) { return frame_lines(scene(world::view(world))); }

}  // namespace hop::render
