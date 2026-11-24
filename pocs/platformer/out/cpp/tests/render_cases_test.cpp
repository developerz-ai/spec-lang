// Projected by hand from the @cases of asset_name, bob, draw_text (src/hop/render.slg). Stand-in for spec-lang build output; change the spec, not this file.
#include "harness.hpp"
#include "hop/render/scene.hpp"
#include "hop/render/text.hpp"

using namespace hop::render;

static_assert(bob(0) == -32 && bob(16) == 0 && bob(32) == 32 && bob(48) == 0);
static_assert(asset_name(Asset::FlagMesh) == "flag");

TEST(asset_name_cases) {
  CHECK_EQ(asset_name(Asset::PipMesh), "pip");
  CHECK_EQ(asset_name(Asset::CrawlerMesh), "crawler");
  CHECK_EQ(asset_name(Asset::GemMesh), "gem");
  CHECK_EQ(asset_name(Asset::FlagMesh), "flag");
}

TEST(bob_cases) {
  CHECK_EQ(bob(0), -32);
  CHECK_EQ(bob(16), 0);
  CHECK_EQ(bob(32), 32);
  CHECK_EQ(bob(48), 0);
}

TEST(draw_text_cases) {
  CHECK_EQ(draw_text(Cube{{{0, 0, 0}, {64, 64, 64}}, {1, 2, 3}}), "cube 0,0,0 64,64,64 1,2,3");
  CHECK_EQ(draw_text(Mesh{Asset::PipMesh, {1, 2, 3}, 90}), "mesh pip 1,2,3 90");
}

// The declared world, never started: the frame of the meadow's start without its crawler.
TEST(frame_lines_order) {
  const std::vector<std::string> lines = dump(hop::world::World{});
  CHECK_EQ(lines.size(), std::size_t{18});
  CHECK_EQ(lines[0], "sky 135,206,235");
  CHECK_EQ(lines[1], "camera 0,704,2048 0,192,0");
  CHECK_EQ(lines[2], "time 0");
  CHECK_EQ(lines[3], "cube -512,-256,-512 1024,0,512 98,170,80");
  CHECK_EQ(lines[4], "wires -512,-256,-512 1024,0,512 30,40,30");
  CHECK_EQ(lines[11], "mesh gem 2048,352,0 0");
  CHECK_EQ(lines[14], "mesh flag 5120,0,0 0");
  CHECK_EQ(lines[15], "mesh pip 0,0,0 0");
  CHECK_EQ(lines[16], "label 16 12 20 255,255,255 gems 0/3");
  CHECK_EQ(lines[17], "label 16 38 20 255,255,255 lives 3");
}
