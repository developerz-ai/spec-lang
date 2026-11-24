// Flag, the goal of Skyhop: a striped pole on a stone foot with a gold ball on top and a red
// pennant flying toward +X. Fits 1.0 x 1.0 x 3.0 m (X front, Y left, Z up), origin at the centre
// of the foot. One static body, vertex colour only: one draw call.

import { defineAsset, dmath, type Mesh, MeshBuilder, merge, type Vec3 } from "@developerz.ai/ts23d";
import { blob, faceGameX, mix, paint, rgb, smooth, turned } from "../_lib/shapes.ts";

const STONE = rgb(0xb9b4a8);
const STONE_DARK = rgb(0x8d8778);
const POLE_LIGHT = rgb(0xfaf6ec);
const POLE_RED = rgb(0xe23d4b);
const GOLD = rgb(0xffcf3a);
const CLOTH = rgb(0xe8303f);
const CLOTH_DARK = rgb(0xb01f3a);
const CLOTH_BAND = rgb(0xfff3d6);

const POLE_R = 0.045;
const POLE_BOTTOM = 0.18;
const POLE_TOP = 2.86;
const BANDS = 8;

/** A low stepped foot; its top is a shallow cone, so no face is exactly level. */
function foot(): Mesh {
  const stone = turned({
    profile: [
      [0.49, 0],
      [0.49, 0.07],
      [0.4, 0.1],
      [0.22, 0.12],
      [0.2, 0.19],
      [0.09, 0.22],
    ],
    color: STONE,
    segments: 16,
  });
  return paint(stone, (p) => mix(STONE_DARK, STONE, smooth(0.0, 0.11, p[2])));
}

/** The pole in bands of alternating colour, each its own lathe so the bands stay crisp. */
function pole(): Mesh[] {
  const step = (POLE_TOP - POLE_BOTTOM) / BANDS;
  return Array.from({ length: BANDS }, (_, i) =>
    turned({
      profile: [
        [POLE_R, 0],
        [POLE_R, step],
      ],
      color: i % 2 === 0 ? POLE_LIGHT : POLE_RED,
      segments: 8,
      at: [0, 0, POLE_BOTTOM + i * step],
    }),
  );
}

/**
 * A triangular pennant from the pole to a tip at +X, rippling in Y. Both faces are emitted
 * (raylib culls back faces), 6 mm apart. Every column of quads has its own vertices, so the
 * cream band keeps hard edges.
 */
function pennant(): Mesh {
  const top = 2.84;
  const bottom = 1.98;
  const reach = 0.455;
  const columns = 10;
  const rows = 4;
  const b = new MeshBuilder();
  const at = (u: number, v: number): Vec3 => {
    const mid = (top + bottom) / 2 - 0.08 * u * u;
    const half = ((top - bottom) / 2) * (1 - u * 0.94);
    const wave = 0.07 * u * dmath.sin(u * 7.2 + v * 0.8);
    return [POLE_R + u * reach, wave, mid - half + v * 2 * half];
  };
  const colorOf = (column: number): Vec3 =>
    column === 2 || column === 3 ? CLOTH_BAND : mix(CLOTH, CLOTH_DARK, smooth(0.4, 1, column / columns));
  for (const face of [1, -1]) {
    const edge = (u: number, color: Vec3): number[] =>
      Array.from({ length: rows + 1 }, (_, j) => {
        const v = j / rows;
        const p = at(u, v);
        // Normal from the ripple's slope along X; the cloth is close to the XZ plane.
        const e = 0.01;
        const slope = (at(u + e, v)[1] - at(u - e, v)[1]) / (2 * e * reach);
        const len = Math.sqrt(1 + slope * slope);
        return b.vertex({
          p: [p[0], p[1] + face * 0.003, p[2]],
          n: [(-slope / len) * face, face / len, 0],
          uv: [0, 0],
          color,
        });
      });
    for (let i = 0; i < columns; i++) {
      const near = edge(i / columns, colorOf(i));
      const far = edge((i + 1) / columns, colorOf(i));
      for (let j = 0; j < rows; j++) {
        const [a, c, d, e] = [near[j] ?? 0, far[j] ?? 0, far[j + 1] ?? 0, near[j + 1] ?? 0];
        // The +Y face is seen from +Y: counter-clockwise there is a, e, d, c.
        if (face > 0) b.quad(a, e, d, c);
        else b.quad(a, c, d, e);
      }
    }
  }
  return b.build();
}

export const flag = defineAsset({
  name: "flag",
  kind: "prop",
  description: "Flag, the goal of Skyhop",
  budget: { triangles: [900, 900, 900], draws: 1 },
  build(ctx) {
    ctx.part(
      "flag",
      faceGameX(
        merge([foot(), ...pole(), blob({ at: [0, 0, POLE_TOP + 0.07], r: [0.07, 0.07, 0.07], color: GOLD, segments: 10 }), pennant()]),
      ),
    );
  },
});
