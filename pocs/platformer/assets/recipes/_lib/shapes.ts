// Shared shape and colour helpers for the Skyhop recipes. Not an asset.
// ts23d space: metres, Z up, +X front, +Y left. The GLB root rotates that to glTF Y-up, so the
// game sees +X front, +Y up and game Z = -recipe Y.

import { applyAffine, dmath, frame, lathe, type Mesh, place, type Vec2, type Vec3 } from "@developerz.ai/ts23d";

/**
 * A display colour, written to COLOR_0 as-is. raylib's glTF loader copies COLOR_0 to the vertex
 * colour without any transfer function and the lit shader works in display space, so these are
 * sRGB bytes, not ts23d's linear `hex()`.
 */
export const rgb = (value: number): Vec3 => [((value >> 16) & 0xff) / 255, ((value >> 8) & 0xff) / 255, (value & 0xff) / 255];

export const mix = (a: Vec3, b: Vec3, t: number): Vec3 => [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];

export const smooth = (lo: number, hi: number, x: number): number => {
  const t = Math.max(0, Math.min(1, (x - lo) / (hi - lo)));
  return t * t * (3 - 2 * t);
};

/** Unit-sphere profile, pole to pole, `rings` bands. */
const sphereProfile = (rings: number): Vec2[] =>
  Array.from({ length: rings + 1 }, (_, i): Vec2 => {
    const a = -Math.PI / 2 + (Math.PI * i) / rings;
    return i === 0 ? [0, -1] : i === rings ? [0, 1] : [dmath.cos(a), dmath.sin(a)];
  });

export interface BlobOptions {
  readonly at: Vec3;
  /** Half extents along X, Y, Z. */
  readonly r: Vec3;
  readonly color: Vec3;
  readonly segments?: number;
  readonly rings?: number;
  readonly yaw?: number;
  readonly pitch?: number;
  readonly roll?: number;
}

/** A smooth ellipsoid centred on `at`. */
export function blob(o: BlobOptions): Mesh {
  const unit = lathe({
    profile: sphereProfile(o.rings ?? 6),
    segments: o.segments ?? 12,
    color: o.color,
    creaseDeg: 180,
  });
  return place(unit, {
    at: o.at,
    scale: o.r,
    ...(o.yaw === undefined ? {} : { yaw: o.yaw }),
    ...(o.pitch === undefined ? {} : { pitch: o.pitch }),
    ...(o.roll === undefined ? {} : { roll: o.roll }),
  });
}

export interface TurnedOptions {
  /** (radius, z) pairs, bottom to top. */
  readonly profile: readonly Vec2[];
  readonly color: Vec3;
  readonly segments?: number;
  readonly at?: Vec3;
  readonly scale?: Vec3;
  readonly pitch?: number;
  readonly roll?: number;
  readonly creaseDeg?: number;
}

/** A lathed solid, optionally squashed and leaned (pitch: forward, roll: toward -Y). */
export function turned(o: TurnedOptions): Mesh {
  const solid = lathe({
    profile: o.profile,
    segments: o.segments ?? 10,
    color: o.color,
    caps: true,
    ...(o.creaseDeg === undefined ? {} : { creaseDeg: o.creaseDeg }),
  });
  return place(solid, {
    at: o.at ?? [0, 0, 0],
    ...(o.scale === undefined ? {} : { scale: o.scale }),
    ...(o.pitch === undefined ? {} : { pitch: o.pitch }),
    ...(o.roll === undefined ? {} : { roll: o.roll }),
  });
}

/** Recolour every vertex from its position and normal. */
export function paint(mesh: Mesh, color: (p: Vec3, n: Vec3) => Vec3): Mesh {
  const colors = new Float32Array(mesh.colors.length);
  for (let i = 0; i < mesh.positions.length; i += 3) {
    const c = color(
      [mesh.positions[i] ?? 0, mesh.positions[i + 1] ?? 0, mesh.positions[i + 2] ?? 0],
      [mesh.normals[i] ?? 0, mesh.normals[i + 1] ?? 0, mesh.normals[i + 2] ?? 1],
    );
    colors.set(c, i);
  }
  return { ...mesh, colors };
}

/**
 * The last step of a recipe built facing +X: an exact quarter turn about Z, to face +Y. ts23d
 * writes recipe +X to glTF +Z (glTF's front) and recipe +Y to glTF +X; Skyhop runs along the
 * game's +X, so the turn makes the GLB face +X with no rotation at draw time.
 */
export const faceGameX = (mesh: Mesh): Mesh => applyAffine(mesh, frame([0, 1, 0], [-1, 0, 0], [0, 0, 1]));
