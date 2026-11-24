// Projected by hand from Solid, Mover, Gem, Crawler, Level (src/hop/model.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Box } from "../geom/box.ts";
import type { Vec3 } from "../geom/vec3.ts";
import type { Rgb } from "./rgb.ts";

export type Solid = { readonly box: Box; readonly color: Rgb };

/** A one-way platform: it carries whoever lands on it from above and never blocks from the side or below. */
export type Mover = {
  readonly no: number;
  readonly home: Box;
  readonly step: Vec3;
  /** `1..=1024` */
  readonly span: number;
  readonly color: Rgb;
};

export type Gem = { readonly no: number; readonly at: Vec3 };

/** A crawler patrols along x between lo and hi. `dir` is `-1..=1`, `speed` is `1..=8`. */
export type Crawler = {
  readonly no: number;
  readonly at: Vec3;
  readonly lo: number;
  readonly hi: number;
  readonly dir: number;
  readonly speed: number;
};

export type Level = {
  readonly name: string;
  readonly start: Vec3;
  readonly solids: readonly Solid[];
  readonly movers: readonly Mover[];
  readonly gems: readonly Gem[];
  readonly crawlers: readonly Crawler[];
  readonly goal: Vec3;
};
