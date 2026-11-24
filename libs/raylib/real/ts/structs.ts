// The `raylib` library's runtime for TypeScript (libs/raylib/real/ts): the program's values as raylib's C structs.
// The card has no f32 and no struct layout, so the edge converts by shape: a record of three Ints is a `Vector3`
// in 1/256 of a unit; a record of three or four Ints is a `Color` (alpha 255 when absent); a record of two such
// vectors is a `BoundingBox`. Sizes and offsets are LP64's, asserted against raylib.h in shim.c.

import { type Pointer, toArrayBuffer } from "bun:ffi";

export const SIZE = { shader: 16, model: 120, material: 40, renderTexture: 44, image: 24 } as const;
const MODEL_MATERIAL_COUNT = 68;
const MODEL_MATERIALS = 80;
export const RENDER_TEXTURE_TEXTURE = 4;
export const TEXTURE_SIZE = 20;

/** The fields of a record value, in declared order. */
function fields(v: unknown): readonly unknown[] {
  return Object.values(v as Record<string, unknown>);
}

/** A whole number of the program as a JS number. */
export function num(v: unknown): number {
  return Number(v as bigint);
}

/** `(float)v / 256.0f` per coordinate: exact for every coordinate a program keeps within 2^24. */
export function putVector3(out: Float32Array, v: unknown, at = 0): void {
  const [x, y, z] = fields(v);
  out[at] = num(x) / 256;
  out[at + 1] = num(y) / 256;
  out[at + 2] = num(z) / 256;
}

/** `Color{r, g, b, a}`, alpha 255 when the record has three fields. */
export function putColor(out: Uint8Array, c: unknown): void {
  const [r, g, b, a] = fields(c);
  out[0] = num(r);
  out[1] = num(g);
  out[2] = num(b);
  out[3] = a === undefined ? 255 : num(a);
}

/** `BoundingBox{min, max}`: six floats. */
export function putBoundingBox(out: Float32Array, b: unknown): void {
  const [lo, hi] = fields(b);
  putVector3(out, lo, 0);
  putVector3(out, hi, 3);
}

/** A box as `DrawCube` takes it: the centre `(lo + hi) / 512` per axis, then width, height and depth. */
export function centreAndSize(out: Float32Array, b: unknown): readonly [number, number, number] {
  const [lo, hi] = fields(b).map(fields) as [readonly unknown[], readonly unknown[]];
  const size = [0, 1, 2].map((i) => (num(hi[i]) - num(lo[i])) / 256) as [number, number, number];
  for (const i of [0, 1, 2]) out[i] = (num(lo[i]) + num(hi[i])) / 512;
  return size;
}

/** `Camera3D{eye, target, up = {0, 1, 0}, fovy = 45.0f, projection = CAMERA_PERSPECTIVE}`. */
export function putCamera(out: DataView, eye: unknown, target: unknown): void {
  const e = new Float32Array(3);
  const t = new Float32Array(3);
  putVector3(e, eye);
  putVector3(t, target);
  const floats = [...e, ...t, 0, 1, 0, 45];
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
