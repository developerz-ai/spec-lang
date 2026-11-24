// Projected by hand from Draw, Label, Frame (src/hop/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Box } from "../geom/box.ts";
import type { Vec3 } from "../geom/vec3.ts";
import type { Rgb } from "../model/rgb.ts";
import type { Asset } from "./asset.ts";

export type Draw =
  | { readonly kind: "Cube"; readonly box: Box; readonly color: Rgb }
  | { readonly kind: "Wires"; readonly box: Box; readonly color: Rgb }
  | { readonly kind: "Mesh"; readonly asset: Asset; readonly at: Vec3; readonly yaw: number };

export type Label = {
  readonly x: number;
  readonly y: number;
  readonly size: number;
  readonly text: string;
  readonly color: Rgb;
};

/** Time is the tick count, an exact integer; the shader divides it into seconds. */
export type Frame = {
  readonly sky: Rgb;
  readonly eye: Vec3;
  readonly target: Vec3;
  readonly time: number;
  readonly world: readonly Draw[];
  readonly labels: readonly Label[];
};
