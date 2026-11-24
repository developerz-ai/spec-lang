// Crawler, the patrolling enemy of Skyhop: a domed beetle with a spotted purple shell, a dark
// head with yellow eyes, two club antennae and six stubby legs. An original design. Fits
// 0.75 x 0.75 x 0.5 m (X front, Y left, Z up), origin at the feet, facing +X. One draw call.

import { defineAsset, type Mesh, merge, type Vec3 } from "@developerz.ai/ts23d";
import { blob, faceGameX, mix, paint, rgb, smooth, turned } from "../_lib/shapes.ts";

const SHELL = rgb(0x8a4fd6);
const SHELL_DARK = rgb(0x4a2488);
const SPOT = rgb(0xd9b8ff);
const HIDE = rgb(0x2a2140);
const LEG = rgb(0x3a2d5c);
const EYE = rgb(0xffd23f);
const PUPIL = rgb(0x1a1226);
const JAW = rgb(0xf2ead8);

/** The wing cases: a dome with a flat belly, a dark seam down the middle and a dark rim. */
function shell(): Mesh {
  const dome = turned({
    profile: [
      [0, -0.2],
      [0.8, -0.16],
      [1, 0],
      [0.95, 0.3],
      [0.8, 0.6],
      [0.56, 0.82],
      [0.29, 0.95],
      [0, 1],
    ],
    color: SHELL,
    segments: 20,
    at: [-0.045, 0, 0.13],
    scale: [0.33, 0.3, 0.3],
    creaseDeg: 180,
  });
  return paint(dome, (p) => {
    const seam = 1 - smooth(0.0, 0.03, Math.abs(p[1]));
    const rim = 1 - smooth(0.11, 0.19, p[2]);
    return mix(SHELL, SHELL_DARK, Math.max(seam, rim));
  });
}

/** A pale spot lying on the shell at (x, y); z is where the dome is there. */
function spot(x: number, y: number, r: number): Mesh {
  const u = (x + 0.045) / 0.33;
  const v = y / 0.3;
  const z = 0.13 + 0.3 * Math.sqrt(Math.max(0, 1 - u * u - v * v));
  return blob({ at: [x, y, z - 0.012], r: [r, r, 0.03], color: SPOT, segments: 8, rings: 4 });
}

function leg(x: number, side: number): Mesh {
  return blob({ at: [x, side * 0.295, 0.065], r: [0.055, 0.08, 0.065], color: LEG, segments: 8, rings: 5 });
}

/** A club antenna leaning forward and outward, tip at the top of the bounding box. */
function antenna(side: number): Mesh[] {
  const base: Vec3 = [0.25, side * 0.07, 0.22];
  const tip: Vec3 = [0.33, side * 0.16, 0.455];
  const stalk = turned({
    profile: [
      [0.022, 0],
      [0.016, 0.26],
    ],
    color: HIDE,
    segments: 6,
    at: base,
    pitch: 0.33,
    roll: -side * 0.36,
  });
  return [stalk, blob({ at: tip, r: [0.045, 0.045, 0.045], color: SPOT, segments: 8, rings: 5 })];
}

function face(side: number): Mesh[] {
  const y = side * 0.105;
  return [
    blob({ at: [0.3, y, 0.205], r: [0.055, 0.06, 0.068], color: EYE, segments: 10 }),
    blob({ at: [0.338, y * 1.02, 0.198], r: [0.024, 0.032, 0.038], color: PUPIL, segments: 8, rings: 4 }),
    // A pale mandible under each eye.
    blob({ at: [0.335, side * 0.06, 0.085], r: [0.04, 0.03, 0.035], color: JAW, segments: 8, rings: 4 }),
  ];
}

export const crawler = defineAsset({
  name: "crawler",
  kind: "prop",
  description: "Crawler, the patrolling beetle of Skyhop",
  budget: { triangles: [1800, 1800, 1800], draws: 1 },
  build(ctx) {
    ctx.part(
      "body",
      faceGameX(
        merge([
          shell(),
          spot(-0.02, 0.13, 0.06),
          spot(-0.02, -0.13, 0.06),
          spot(-0.2, 0.1, 0.045),
          spot(-0.2, -0.1, 0.045),
          spot(0.12, 0.08, 0.04),
          spot(0.12, -0.08, 0.04),
          blob({ at: [0.24, 0, 0.15], r: [0.13, 0.17, 0.115], color: HIDE, segments: 14, rings: 8 }),
          ...face(1),
          ...face(-1),
          ...antenna(1),
          ...antenna(-1),
          ...[0.16, -0.04, -0.24].flatMap((x) => [leg(x, 1), leg(x, -1)]),
        ]),
      ),
    );
  },
});
