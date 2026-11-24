// Projected by hand from sky, edge, ink, eye_offset, look_height, bob, block, banner, scene (src/hop/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Box } from "../geom/box.ts";
import { triangle } from "../geom/scalar.ts";
import { plus, type Vec3 } from "../geom/vec3.ts";
import type { Mode } from "../model/player.ts";
import { type Rgb, rgb } from "../model/rgb.ts";
import { moverBox } from "../physics/bodies.ts";
import type { View } from "../world/world.ts";
import type { Draw, Frame, Label } from "./frame.ts";

const sky: Rgb = rgb(135, 206, 235);
const edge: Rgb = rgb(30, 40, 30);
const ink: Rgb = rgb(255, 255, 255);

/** The camera looks at the hero from the side and a little above, at a fixed distance. */
export const eyeOffset: Vec3 = Object.freeze({ x: 0, y: 512, z: 2048 });
export const lookHeight = 192;

/** A gem bobs a quarter of a metre and turns three degrees a tick. */
export const bob = (t: number): number => triangle(t, 32) * 2 - 32;

function block(draws: Draw[], box: Box, color: Rgb): void {
  draws.push({ kind: "Cube", box, color }, { kind: "Wires", box, color: edge });
}

const banners: Record<Mode, readonly Label[]> = {
  Playing: [],
  Won: [{ x: 380, y: 250, size: 40, text: "level clear", color: ink }],
  Lost: [{ x: 400, y: 250, size: 40, text: "game over", color: ink }],
};

/** Solids, then movers, gems, crawlers, the flag and last the hero, so the order of the frame is fixed. */
export function scene(v: View): Frame {
  const target = plus(v.player.at, { x: 0, y: lookHeight, z: 0 });
  const draws: Draw[] = [];
  for (const s of v.stage.solids) block(draws, s.box, s.color);
  for (const m of v.stage.movers) block(draws, moverBox(m, v.ticks), m.color);
  const lift = bob(v.ticks);
  const spin = (v.ticks * 3) % 360;
  for (const g of v.stage.gems) {
    if (!v.taken.has(g.no))
      draws.push({ kind: "Mesh", asset: "GemMesh", at: { ...g.at, y: g.at.y + lift }, yaw: spin });
  }
  for (const c of v.crawlers) draws.push({ kind: "Mesh", asset: "CrawlerMesh", at: c.at, yaw: c.dir < 0 ? 180 : 0 });
  draws.push({ kind: "Mesh", asset: "FlagMesh", at: v.stage.goal, yaw: 0 });
  draws.push({ kind: "Mesh", asset: "PipMesh", at: v.player.at, yaw: v.player.yaw });
  const hud: Label[] = [
    { x: 16, y: 12, size: 20, text: `gems ${v.taken.size}/${v.stage.gems.length}`, color: ink },
    { x: 16, y: 38, size: 20, text: `lives ${v.lives}`, color: ink },
  ];
  return {
    sky,
    eye: plus(target, eyeOffset),
    target,
    time: v.ticks,
    world: draws,
    labels: [...hud, ...banners[v.mode]],
  };
}
