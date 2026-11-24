// Projected by hand from Asset, Draw, Label, Frame (src/hop/render.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <array>
#include <cstdint>
#include <string>
#include <variant>
#include <vector>

#include "hop/geom/box.hpp"
#include "hop/model/rgb.hpp"

namespace hop::render {

using geom::Int;

// Each mesh is one model file the game loads at start.
enum class Asset : std::uint8_t { PipMesh, CrawlerMesh, GemMesh, FlagMesh };

// The variants in declared order: the order assets load in.
inline constexpr std::array<Asset, 4> assets{Asset::PipMesh, Asset::CrawlerMesh, Asset::GemMesh,
                                             Asset::FlagMesh};

struct Cube {
  geom::Box box;
  model::Rgb color;
  friend constexpr bool operator==(const Cube&, const Cube&) = default;
};
struct Wires {
  geom::Box box;
  model::Rgb color;
  friend constexpr bool operator==(const Wires&, const Wires&) = default;
};
struct Mesh {
  Asset asset;
  geom::Vec3 at;
  Int yaw;  // 0..=359
  friend constexpr bool operator==(const Mesh&, const Mesh&) = default;
};
using Draw = std::variant<Cube, Wires, Mesh>;

struct Label {
  Int x;
  Int y;
  Int size;
  std::string text;
  model::Rgb color;
  friend bool operator==(const Label&, const Label&) = default;
};

// time is the tick count, an exact integer; the shader divides it into seconds.
struct Frame {
  model::Rgb sky;
  geom::Vec3 eye;
  geom::Vec3 target;
  Int time;
  std::vector<Draw> world;
  std::vector<Label> labels;
  friend bool operator==(const Frame&, const Frame&) = default;
};

}  // namespace hop::render
