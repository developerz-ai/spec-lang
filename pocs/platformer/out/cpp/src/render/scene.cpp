// Projected by hand from sky, edge, ink, block, banner, scene (src/hop/render.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "hop/render/scene.hpp"

#include "hop/geom/text.hpp"
#include "hop/physics/bodies.hpp"

namespace hop::render {
namespace {

constexpr model::Rgb sky{135, 206, 235};
constexpr model::Rgb edge{30, 40, 30};
constexpr model::Rgb ink{255, 255, 255};

void block(std::vector<Draw>& draws, const geom::Box& box, model::Rgb color) {
  draws.emplace_back(Cube{box, color});
  draws.emplace_back(Wires{box, edge});
}

void banner(std::vector<Label>& labels, model::Mode m) {
  switch (m) {
    case model::Mode::Playing:
      break;
    case model::Mode::Won:
      labels.push_back({380, 250, 40, "level clear", ink});
      break;
    case model::Mode::Lost:
      labels.push_back({400, 250, 40, "game over", ink});
      break;
  }
}

std::string count_label(std::string_view word, Int n) {
  std::string text(word);
  geom::append_int(text, n);
  return text;
}

}  // namespace

Frame scene(const world::View& v) {
  const geom::Vec3 target = geom::plus(v.player.at, {0, look_height, 0});
  std::vector<Draw> draws;
  // The size of the list is known before it is built: two per block, one per mesh.
  draws.reserve(2 * (v.stage.solids.size() + v.stage.movers.size()) + v.stage.gems.size() +
                v.crawlers.size() + 2);
  for (const model::Solid& s : v.stage.solids) {
    block(draws, s.box, s.color);
  }
  for (const model::Mover& m : v.stage.movers) {
    block(draws, physics::mover_box(m, v.ticks), m.color);
  }
  for (const model::Gem& g : v.stage.gems) {
    if (!v.taken.contains(g.no)) {
      draws.emplace_back(
          Mesh{Asset::GemMesh, {g.at.x, g.at.y + bob(v.ticks), g.at.z}, geom::floor_mod(v.ticks * 3, 360)});
    }
  }
  for (const model::Crawler& c : v.crawlers) {
    draws.emplace_back(Mesh{Asset::CrawlerMesh, c.at, c.dir < 0 ? 180 : 0});
  }
  draws.emplace_back(Mesh{Asset::FlagMesh, v.stage.goal, 0});
  draws.emplace_back(Mesh{Asset::PipMesh, v.player.at, v.player.yaw});

  std::vector<Label> labels;
  labels.reserve(3);
  std::string gems = count_label("gems ", static_cast<Int>(v.taken.size()));
  gems += '/';
  geom::append_int(gems, static_cast<Int>(v.stage.gems.size()));
  labels.push_back({16, 12, 20, std::move(gems), ink});
  labels.push_back({16, 38, 20, count_label("lives ", v.lives), ink});
  banner(labels, v.mode);
  return {sky, geom::plus(target, eye_offset), target, v.ticks, std::move(draws), std::move(labels)};
}

}  // namespace hop::render
