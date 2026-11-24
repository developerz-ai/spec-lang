// Projected by hand from the C layouts at the edge: Sub to float, Vec3 to Vector3, Rgb to Color, Box to BoundingBox, and raylib 5.5's own structs (docs/contract.md section 5, vendor/raylib/include/raylib.h). Stand-in for spec-lang build output; change the spec, not this file.
// Sizes and offsets are for LP64 (8-byte pointers), and native/shim.c asserts each of them against raylib.h.

import { type Pointer, toArrayBuffer } from "bun:ffi";
import type { Box } from "../geom/box.ts";
import type { Vec3 } from "../geom/vec3.ts";
import type { Rgb } from "../model/rgb.ts";

export const SIZE = { shader: 16, model: 120, material: 40, renderTexture: 44, image: 24 } as const;
const MODEL_MATERIAL_COUNT = 68;
const MODEL_MATERIALS = 80;
export const RENDER_TEXTURE_TEXTURE = 4;
export const TEXTURE_SIZE = 20;

/** `(float)v / 256.0f` for each of x, y, z: exact for every Sub, and storing to a Float32Array is that cast. */
export function putVector3(out: Float32Array, v: Vec3): void {
  out[0] = v.x / 256;
  out[1] = v.y / 256;
  out[2] = v.z / 256;
}

/** `Color{r, g, b, 255}` */
export function putColor(out: Uint8Array, c: Rgb): void {
  out[0] = c.r;
  out[1] = c.g;
  out[2] = c.b;
  out[3] = 255;
}

/** `BoundingBox{lo, hi}`: six floats. */
export function putBoundingBox(out: Float32Array, b: Box): void {
  out[0] = b.lo.x / 256;
  out[1] = b.lo.y / 256;
  out[2] = b.lo.z / 256;
  out[3] = b.hi.x / 256;
  out[4] = b.hi.y / 256;
  out[5] = b.hi.z / 256;
}

/** `centre = (lo + hi) / 512.0f` per axis; the integer sum is at most 2^21, exact as a float. */
export function putCentre(out: Float32Array, b: Box): void {
  out[0] = (b.lo.x + b.hi.x) / 512;
  out[1] = (b.lo.y + b.hi.y) / 512;
  out[2] = (b.lo.z + b.hi.z) / 512;
}

/** `Camera3D{eye, target, up = {0, 1, 0}, fovy = 45.0f, projection = CAMERA_PERSPECTIVE}`: ten floats and an int. */
export function putCamera(out: DataView, eye: Vec3, target: Vec3): void {
  const floats = [eye.x / 256, eye.y / 256, eye.z / 256, target.x / 256, target.y / 256, target.z / 256, 0, 1, 0, 45];
  for (const [i, f] of floats.entries()) out.setFloat32(i * 4, f, true);
  out.setInt32(40, 0, true);
}

const pointerAt = (struct: Uint8Array, offset: number): Pointer =>
  Number(new DataView(struct.buffer, struct.byteOffset).getBigUint64(offset, true)) as Pointer;

/** `model.materials[i].shader = shader` for every material: 16 bytes copied into raylib's own array. */
export function setMaterialShaders(model: Uint8Array, shader: Uint8Array): void {
  const count = new DataView(model.buffer, model.byteOffset).getInt32(MODEL_MATERIAL_COUNT, true);
  if (count <= 0) return;
  const materials = new Uint8Array(toArrayBuffer(pointerAt(model, MODEL_MATERIALS), 0, count * SIZE.material));
  for (let i = 0; i < count; i++) materials.set(shader, i * SIZE.material);
}

/** The `width * height * 4` bytes of an R8G8B8A8 `Image`, read in place. */
export function imagePixels(image: Uint8Array): Uint8Array {
  const view = new DataView(image.buffer, image.byteOffset);
  return new Uint8Array(toArrayBuffer(pointerAt(image, 0), 0, view.getInt32(8, true) * view.getInt32(12, true) * 4));
}

/** A NUL-terminated UTF-8 copy, for a `const char *`. */
export const cString = (text: string): Uint8Array => new TextEncoder().encode(`${text}\0`);
