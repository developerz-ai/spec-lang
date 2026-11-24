// Projected by hand from the @cases of asset_name, bob, draw_text (src/hop/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { expect, test } from "bun:test";
import { box } from "../../src/geom/box.ts";
import { vec3 } from "../../src/geom/vec3.ts";
import { rgb } from "../../src/model/rgb.ts";
import { type Asset, assetName } from "../../src/render/asset.ts";
import type { Draw } from "../../src/render/frame.ts";
import { bob } from "../../src/render/scene.ts";
import { drawText } from "../../src/render/text.ts";

test.each<[Asset, string]>([
  ["PipMesh", "pip"],
  ["CrawlerMesh", "crawler"],
  ["GemMesh", "gem"],
  ["FlagMesh", "flag"],
])("asset_name(%s) -> %s", (asset, want) => {
  expect(assetName(asset)).toBe(want);
});

test.each([
  [0, -32],
  [16, 0],
  [32, 32],
  [48, 0],
])("bob(%i) -> %i", (t, want) => {
  expect(bob(t)).toBe(want);
});

test.each<[Draw, string]>([
  [{ kind: "Cube", box: box(vec3(0, 0, 0), vec3(64, 64, 64)), color: rgb(1, 2, 3) }, "cube 0,0,0 64,64,64 1,2,3"],
  [{ kind: "Mesh", asset: "PipMesh", at: vec3(1, 2, 3), yaw: 90 }, "mesh pip 1,2,3 90"],
])("draw_text %#", (d, want) => {
  expect(drawText(d)).toBe(want);
});
