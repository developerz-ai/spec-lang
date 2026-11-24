// Projected by hand from grass, stone, wood, q, at, slab (src/hop/levels.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Vec3 } from "../geom/vec3.ts";
import type { Solid } from "../model/level.ts";
import { type Rgb, rgb } from "../model/rgb.ts";

export const grass: Rgb = rgb(98, 170, 80);
export const stone: Rgb = rgb(150, 150, 160);
export const wood: Rgb = rgb(210, 150, 60);

/** Levels are laid out on a quarter-metre grid: 64 sub-metres. */
export const q = (n: number): number => n * 64;

export const at = (x: number, y: number, z: number): Vec3 => ({ x: q(x), y: q(y), z: q(z) });

export const slab = (from: Vec3, to: Vec3, color: Rgb): Solid => ({ box: { lo: from, hi: to }, color });
