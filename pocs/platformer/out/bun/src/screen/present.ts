// Projected by hand from rule present (src/hop/screen.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { scene } from "../render/scene.ts";
import type { View } from "../world/world.ts";
import type { Canvas } from "./canvas.ts";

/** The frame as data goes to raylib in its own order: background, 3D pass under the lit shader, then the HUD. */
export function present(to: Canvas, v: View): void {
  const f = scene(v);
  to.clearBackground(f.sky);
  to.beginMode3d(f.eye, f.target);
  to.beginLit(f.time);
  for (const d of f.world) {
    switch (d.kind) {
      case "Cube":
        to.drawCube(d.box, d.color);
        break;
      case "Wires":
        to.drawCubeWires(d.box, d.color);
        break;
      case "Mesh":
        to.drawModel(d.asset, d.at, d.yaw);
        break;
    }
  }
  to.endLit();
  to.endMode3d();
  for (const l of f.labels) to.drawText(l.text, l.x, l.y, l.size, l.color);
}
