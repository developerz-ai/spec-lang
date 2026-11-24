// Projected by hand from clear_background, begin_mode_3d, end_mode_3d, begin_lit, end_lit, draw_cube, draw_cube_wires, draw_model, draw_text (src/hop/screen.slg, docs/contract.md section 5). Stand-in for spec-lang build output; change the spec, not this file.

import type { Box } from "../geom/box.ts";
import type { Vec3 } from "../geom/vec3.ts";
import type { Rgb } from "../model/rgb.ts";
import type { Asset } from "../render/asset.ts";
import type { Assets } from "./assets.ts";
import type { Raylib } from "./raylib.ts";
import { cString, putCamera, putCentre, putColor, putVector3 } from "./structs.ts";

const SHADER_UNIFORM_FLOAT = 0;

/** The `screen.write` rules, each one C call. */
export type Canvas = {
  readonly clearBackground: (color: Rgb) => void;
  readonly beginMode3d: (eye: Vec3, target: Vec3) => void;
  readonly endMode3d: () => void;
  readonly beginLit: (time: number) => void;
  readonly endLit: () => void;
  readonly drawCube: (box: Box, color: Rgb) => void;
  readonly drawCubeWires: (box: Box, color: Rgb) => void;
  readonly drawModel: (asset: Asset, at: Vec3, yaw: number) => void;
  readonly drawText: (text: string, x: number, y: number, size: number, color: Rgb) => void;
};

/**
 * The argument structs are written into buffers allocated once here: a frame allocates nothing at the edge.
 * Licence: every declaration is `@uses(screen.write)` only, so no call keeps its arguments.
 */
export function canvas(rl: Raylib, assets: Assets): Canvas {
  const { c, shim } = rl;
  const fn = {
    clear: rl.fn("ClearBackground"),
    mode3d: rl.fn("BeginMode3D"),
    setValue: rl.fn("SetShaderValue"),
    shaderMode: rl.fn("BeginShaderMode"),
    cube: rl.fn("DrawCube"),
    wires: rl.fn("DrawCubeWires"),
    model: rl.fn("DrawModelEx"),
    text: rl.fn("DrawText"),
  };
  const vector = new Float32Array(3);
  const color = new Uint8Array(4);
  const camera = new DataView(new ArrayBuffer(44));
  const cameraBytes = new Uint8Array(camera.buffer);
  const uniform = new Float32Array(1);
  const yAxis = new Float32Array([0, 1, 0]);
  const fullScale = new Float32Array([1, 1, 1]);
  const white = new Uint8Array([255, 255, 255, 255]);

  return {
    clearBackground(sky) {
      putColor(color, sky);
      shim.skyhop_ClearBackground(fn.clear, color);
    },
    beginMode3d(eye, target) {
      putCamera(camera, eye, target);
      shim.skyhop_BeginMode3D(fn.mode3d, cameraBytes);
    },
    endMode3d: () => c.EndMode3D(),
    beginLit(time) {
      uniform[0] = time;
      shim.skyhop_SetShaderValue(fn.setValue, assets.lit, assets.timeLoc, uniform, SHADER_UNIFORM_FLOAT);
      shim.skyhop_BeginShaderMode(fn.shaderMode, assets.lit);
    },
    endLit: () => c.EndShaderMode(),
    drawCube(box, rgb) {
      putCentre(vector, box);
      putColor(color, rgb);
      const w = (box.hi.x - box.lo.x) / 256;
      shim.skyhop_DrawCube(fn.cube, vector, w, (box.hi.y - box.lo.y) / 256, (box.hi.z - box.lo.z) / 256, color);
    },
    drawCubeWires(box, rgb) {
      putCentre(vector, box);
      putColor(color, rgb);
      const w = (box.hi.x - box.lo.x) / 256;
      shim.skyhop_DrawCubeWires(fn.wires, vector, w, (box.hi.y - box.lo.y) / 256, (box.hi.z - box.lo.z) / 256, color);
    },
    drawModel(asset, at, yaw) {
      putVector3(vector, at);
      shim.skyhop_DrawModelEx(fn.model, assets.models[asset], vector, yAxis, yaw, fullScale, white);
    },
    drawText(text, x, y, size, rgb) {
      putColor(color, rgb);
      shim.skyhop_DrawText(fn.text, cString(text), x, y, size, color);
    },
  };
}
