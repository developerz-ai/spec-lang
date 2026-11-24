// Projected by hand from Box, shifted, around_feet, around_centre, overlaps, touches, over (src/hop/geom.slg). Stand-in for spec-lang build output; change the spec, not this file.

import { plus, type Vec3 } from "./vec3.ts";

/** `lo` is the corner nearest minus infinity on every axis. */
export type Box = { readonly lo: Vec3; readonly hi: Vec3 };

export const box = (lo: Vec3, hi: Vec3): Box => ({ lo, hi });

export const shifted = (b: Box, by: Vec3): Box => ({ lo: plus(b.lo, by), hi: plus(b.hi, by) });

/** A body is placed by the centre of its base: its feet. */
export const aroundFeet = (feet: Vec3, half: Vec3): Box => ({
  lo: { x: feet.x - half.x, y: feet.y, z: feet.z - half.z },
  hi: { x: feet.x + half.x, y: feet.y + half.y, z: feet.z + half.z },
});

/** A gem is placed by its centre. */
export const aroundCentre = (c: Vec3, half: number): Box => ({
  lo: { x: c.x - half, y: c.y - half, z: c.z - half },
  hi: { x: c.x + half, y: c.y + half, z: c.z + half },
});

/** Solids block only when they share volume. */
export const overlaps = (a: Box, b: Box): boolean =>
  a.lo.x < b.hi.x && b.lo.x < a.hi.x && a.lo.y < b.hi.y && b.lo.y < a.hi.y && a.lo.z < b.hi.z && b.lo.z < a.hi.z;

/** A touch, faces included: the same test raylib's CheckCollisionBoxes makes. */
export const touches = (a: Box, b: Box): boolean =>
  a.lo.x <= b.hi.x && b.lo.x <= a.hi.x && a.lo.y <= b.hi.y && b.lo.y <= a.hi.y && a.lo.z <= b.hi.z && b.lo.z <= a.hi.z;

/** The footprints meet strictly. */
export const over = (a: Box, b: Box): boolean =>
  a.lo.x < b.hi.x && b.lo.x < a.hi.x && a.lo.z < b.hi.z && b.lo.z < a.hi.z;
