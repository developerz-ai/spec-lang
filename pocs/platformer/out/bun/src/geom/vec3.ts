// Projected by hand from Sub, unit, Vec3, plus, minus, scaled, vec_text (src/hop/geom.slg). Stand-in for spec-lang build output; change the spec, not this file.

/** `Sub = -1048576..=1048576`: a sub-metre coordinate. Exact as a JS number and as an f32 at the C edge. */
export type Sub = number;

export const unit = 256;

export type Vec3 = { readonly x: Sub; readonly y: Sub; readonly z: Sub };

export const vec3 = (x: Sub, y: Sub, z: Sub): Vec3 => ({ x, y, z });

export const plus = (a: Vec3, b: Vec3): Vec3 => ({ x: a.x + b.x, y: a.y + b.y, z: a.z + b.z });

export const minus = (a: Vec3, b: Vec3): Vec3 => ({ x: a.x - b.x, y: a.y - b.y, z: a.z - b.z });

export const scaled = (v: Vec3, k: number): Vec3 => ({ x: v.x * k, y: v.y * k, z: v.z * k });

export const vecText = (v: Vec3): string => `${v.x},${v.y},${v.z}`;
