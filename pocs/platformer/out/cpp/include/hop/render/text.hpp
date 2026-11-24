// Projected by hand from asset_name, draw_text, frame_lines, dump (src/hop/render.slg). Stand-in for spec-lang build output; change the spec, not this file.
#pragma once

#include <string>
#include <string_view>
#include <vector>

#include "hop/render/frame.hpp"
#include "hop/world/world.hpp"

namespace hop::render {

constexpr std::string_view asset_name(Asset a) {
  switch (a) {
    case Asset::PipMesh:
      return "pip";
    case Asset::CrawlerMesh:
      return "crawler";
    case Asset::GemMesh:
      return "gem";
    case Asset::FlagMesh:
      return "flag";
  }
  return "";
}

std::string draw_text(const Draw& d);

// A frame as text, one line per call the screen will make.
std::vector<std::string> frame_lines(const Frame& f);

// frame_lines(scene(world.view()))
std::vector<std::string> dump(const world::World& world);

}  // namespace hop::render
