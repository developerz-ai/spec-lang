// Projected by hand from body_half .. kill_y (the constants) (src/hop/physics.slg). Stand-in for spec-lang build output; change the spec, not this file.

import type { Vec3 } from "../geom/vec3.ts";

/** Half the width, the full height and half the depth of the hero. */
export const bodyHalf: Vec3 = Object.freeze({ x: 64, y: 192, z: 64 });
export const crawlerHalf: Vec3 = Object.freeze({ x: 96, y: 128, z: 96 });
export const gemHalf = 64;
export const goalHalf: Vec3 = Object.freeze({ x: 64, y: 768, z: 64 });

export const runSpeed = 16;
export const diagonalSpeed = 11;
export const groundAccel = 2;
export const airAccel = 1;
export const gravity = 2;
/** Never more than 48 sub-metres in a tick, less than the thinnest solid. */
export const maxFall = 48;
export const jumpSpeed = 40;
export const hopCut = 16;
export const coyoteTicks = 6;
export const stompBounce = 24;
export const stompLine = 64;
export const killY = -2048;
