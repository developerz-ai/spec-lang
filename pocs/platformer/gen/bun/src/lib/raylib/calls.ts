// The `raylib` library's runtime for TypeScript (libs/raylib/real/ts): one adapter per `foreign c "raylib#Name"`,
// each one C call on the window open now. The argument structs are written into buffers allocated once.

import * as rt from "../../runtime/index.ts";
import { window } from "./screen.ts";
import { centreAndSize, cString, num, putBoundingBox, putCamera, putColor, putVector3 } from "./structs.ts";

const SHADER_UNIFORM_FLOAT = 0;

const vector = new Float32Array(3);
const color = new Uint8Array(4);
const camera = new DataView(new ArrayBuffer(44));
const cameraBytes = new Uint8Array(camera.buffer);
const uniform = new Float32Array(1);
const yAxis = new Float32Array([0, 1, 0]);
const fullScale = new Float32Array([1, 1, 1]);
const white = new Uint8Array([255, 255, 255, 255]);
const first = new Float32Array(6);
const second = new Float32Array(6);

/** The model a `DrawModelEx` names: by its index, or by its variant's position in the sum it is declared as. */
function modelIndex(v: unknown, decl: rt.ExternInfo): number {
  if (typeof v === "bigint") return Number(v);
  let t = decl.params[0]?.type;
  while (t instanceof rt.AliasType) t = t.of;
  return t instanceof rt.SumType ? t.rank((v as { readonly kind: string }).kind) : -1;
}

function cube(symbol: "DrawCube" | "DrawCubeWires", [box, rgb]: readonly unknown[]): null {
  const { rl } = window();
  const [w, h, d] = centreAndSize(vector, box);
  putColor(color, rgb);
  const call = symbol === "DrawCube" ? rl.shim.spec_rl_DrawCube : rl.shim.spec_rl_DrawCubeWires;
  call(rl.fn(symbol), vector, w, h, d, color);
  return null;
}

export const calls: Readonly<Record<string, (args: readonly unknown[], decl: rt.ExternInfo) => unknown>> = {
  ClearBackground([c]) {
    const { rl } = window();
    putColor(color, c);
    rl.shim.spec_rl_ClearBackground(rl.fn("ClearBackground"), color);
    return null;
  },
  BeginMode3D([eye, target]) {
    const { rl } = window();
    putCamera(camera, eye, target);
    rl.shim.spec_rl_BeginMode3D(rl.fn("BeginMode3D"), cameraBytes);
    return null;
  },
  EndMode3D() {
    window().rl.c.EndMode3D();
    return null;
  },
  BeginShaderMode([time]) {
    const { rl, lit, timeLoc } = window();
    uniform[0] = num(time);
    rl.shim.spec_rl_SetShaderValue(rl.fn("SetShaderValue"), lit, timeLoc, uniform, SHADER_UNIFORM_FLOAT);
    rl.shim.spec_rl_BeginShaderMode(rl.fn("BeginShaderMode"), lit);
    return null;
  },
  EndShaderMode() {
    window().rl.c.EndShaderMode();
    return null;
  },
  DrawCube: (args) => cube("DrawCube", args),
  DrawCubeWires: (args) => cube("DrawCubeWires", args),
  DrawModelEx([model, at, yaw], decl) {
    const { rl, models } = window();
    const m = models[modelIndex(model, decl)];
    if (m === undefined) return rt.stop("E-RUNTIME", "DrawModelEx of a model that was not loaded");
    putVector3(vector, at);
    rl.shim.spec_rl_DrawModelEx(rl.fn("DrawModelEx"), m, vector, yAxis, num(yaw), fullScale, white);
    return null;
  },
  DrawText([text, x, y, size, c]) {
    const { rl } = window();
    putColor(color, c);
    rl.shim.spec_rl_DrawText(rl.fn("DrawText"), cString(text as string), num(x), num(y), num(size), color);
    return null;
  },
  IsKeyDown([key]) {
    return window().rl.c.IsKeyDown(num(key));
  },
  CheckCollisionBoxes([a, b]) {
    const { rl } = window();
    putBoundingBox(first, a);
    putBoundingBox(second, b);
    return rl.shim.spec_rl_CheckCollisionBoxes(rl.fn("CheckCollisionBoxes"), first, second);
  },
};
