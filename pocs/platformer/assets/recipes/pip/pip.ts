// Pip, the hero of Skyhop: a small round hopping creature with two tall ears, big eyes, a cream
// belly and yellow feet. An original design. Fits 0.5 x 0.5 x 0.75 m (X front, Y left, Z up),
// origin at the feet, facing +X. One static body, vertex colour only: one draw call.

import { defineAsset, type Mesh, merge, type Vec3 } from "@developerz.ai/ts23d";
import { blob, faceGameX, mix, paint, rgb, smooth, turned } from "../_lib/shapes.ts";

const FUR = rgb(0xff7a2f);
const FUR_DARK = rgb(0xd9481c);
const BELLY = rgb(0xfff0cf);
const FOOT = rgb(0xffcf3a);
const EYE = rgb(0xffffff);
const PUPIL = rgb(0x241a33);
const NOSE = rgb(0x8a2d1a);
const EAR_IN = rgb(0xffb4a0);

const BELLY_AT: Vec3 = [0.2, 0, 0.2];

/** Orange fur, darker toward the back and crown, with a cream patch on the front. */
function body(): Mesh {
  const mass = blob({ at: [0, 0, 0.3], r: [0.215, 0.225, 0.24], color: FUR, segments: 20, rings: 12 });
  return paint(mass, (p) => {
    const d = Math.hypot(p[0] - BELLY_AT[0], (p[1] - BELLY_AT[1]) * 1.15, p[2] - BELLY_AT[2]);
    const fur = mix(FUR, FUR_DARK, smooth(0.05, -0.22, p[0]) * 0.75);
    return mix(BELLY, fur, smooth(0.15, 0.19, d));
  });
}

/** A tall ear leaning back and outward; `side` is +1 for the left (+Y) ear. */
function ear(side: number): Mesh {
  const shell = turned({
    profile: [
      [0, 0],
      [0.058, 0.02],
      [0.066, 0.1],
      [0.05, 0.2],
      [0.024, 0.3],
      [0, 0.318],
    ],
    color: FUR,
    segments: 10,
    at: [-0.03, side * 0.105, 0.47],
    scale: [0.6, 1.15, 1],
    pitch: -0.22,
    roll: -side * 0.42,
    creaseDeg: 180,
  });
  // The front (+X) face of the ear is the pink inside; the tip is dark.
  return paint(shell, (p, n) => {
    const inner = mix(FUR, EAR_IN, smooth(0.55, 0.85, n[0]) * smooth(0.5, 0.56, p[2]));
    return mix(inner, FUR_DARK, smooth(0.67, 0.73, p[2]));
  });
}

function eye(side: number): Mesh[] {
  const y = side * 0.096;
  return [
    blob({ at: [0.172, y, 0.38], r: [0.042, 0.066, 0.084], color: EYE, segments: 12, rings: 8 }),
    blob({ at: [0.2, y * 0.96, 0.378], r: [0.026, 0.036, 0.05], color: PUPIL, segments: 10 }),
    blob({ at: [0.218, y * 0.96 + 0.012, 0.398], r: [0.012, 0.012, 0.014], color: EYE, segments: 6, rings: 4 }),
  ];
}

export const pip = defineAsset({
  name: "pip",
  kind: "prop",
  description: "Pip, the hopping hero of Skyhop",
  budget: { triangles: [2000, 2000, 2000], draws: 1 },
  build(ctx) {
    ctx.part(
      "body",
      faceGameX(
        merge([
          body(),
          ear(1),
          ear(-1),
          ...eye(1),
          ...eye(-1),
          blob({ at: [0.222, 0, 0.3], r: [0.028, 0.04, 0.026], color: NOSE, segments: 8, rings: 4 }),
          // Feet: flat on the ground, toes forward.
          blob({ at: [0.05, 0.115, 0.045], r: [0.125, 0.075, 0.045], color: FOOT }),
          blob({ at: [0.05, -0.115, 0.045], r: [0.125, 0.075, 0.045], color: FOOT }),
          // Paws at the sides and a tuft of a tail.
          blob({ at: [0.06, 0.2, 0.25], r: [0.055, 0.05, 0.06], color: FUR_DARK, segments: 8, rings: 5 }),
          blob({ at: [0.06, -0.2, 0.25], r: [0.055, 0.05, 0.06], color: FUR_DARK, segments: 8, rings: 5 }),
          blob({ at: [-0.19, 0, 0.19], r: [0.06, 0.06, 0.06], color: BELLY, segments: 8, rings: 5 }),
        ]),
      ),
    );
  },
});
