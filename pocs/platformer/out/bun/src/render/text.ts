// Projected by hand from rgb_text, draw_text, frame_lines, dump (src/hop/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { vecText } from "../geom/vec3.ts";
import type { Rgb } from "../model/rgb.ts";
import { view, type World } from "../world/world.ts";
import { assetName } from "./asset.ts";
import type { Draw, Frame } from "./frame.ts";
import { scene } from "./scene.ts";

const rgbText = (c: Rgb): string => `${c.r},${c.g},${c.b}`;

export function drawText(d: Draw): string {
  switch (d.kind) {
    case "Cube":
      return `cube ${vecText(d.box.lo)} ${vecText(d.box.hi)} ${rgbText(d.color)}`;
    case "Wires":
      return `wires ${vecText(d.box.lo)} ${vecText(d.box.hi)} ${rgbText(d.color)}`;
    case "Mesh":
      return `mesh ${assetName(d.asset)} ${vecText(d.at)} ${d.yaw}`;
  }
}

/** A frame as text, one line per call the screen will make. */
export const frameLines = (f: Frame): string[] => [
  `sky ${rgbText(f.sky)}`,
  `camera ${vecText(f.eye)} ${vecText(f.target)}`,
  `time ${f.time}`,
  ...f.world.map(drawText),
  ...f.labels.map((l) => `label ${l.x} ${l.y} ${l.size} ${rgbText(l.color)} ${l.text}`),
];

export const dump = (w: World): string[] => frameLines(scene(view(w)));
