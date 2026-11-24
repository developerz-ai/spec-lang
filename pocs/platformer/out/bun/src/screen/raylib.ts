// Projected by hand from the foreign c "raylib#..." binding: the library and the shim, opened through bun:ffi (src/hop/screen.slg, docs/contract.md sections 3 and 5). Stand-in for spec-lang build output; change the spec, not this file.

import { dlopen, type Pointer } from "bun:ffi";
import shimFile from "../../build/libskyhop_shim.so" with { type: "file" };

/** Bound only for its address: the call is by value, so it goes through the shim. */
const byValue = { args: [], returns: "void" } as const;

function openLibrary(path: string) {
  return dlopen(path, {
    // Scalars and pointers only: bun:ffi calls these itself.
    SetTraceLogLevel: { args: ["i32"], returns: "void" },
    SetConfigFlags: { args: ["u32"], returns: "void" },
    InitWindow: { args: ["i32", "i32", "ptr"], returns: "void" },
    IsWindowReady: { args: [], returns: "bool" },
    WindowShouldClose: { args: [], returns: "bool" },
    CloseWindow: { args: [], returns: "void" },
    SetTargetFPS: { args: ["i32"], returns: "void" },
    BeginDrawing: { args: [], returns: "void" },
    EndDrawing: { args: [], returns: "void" },
    EndMode3D: { args: [], returns: "void" },
    EndShaderMode: { args: [], returns: "void" },
    EndTextureMode: { args: [], returns: "void" },
    IsKeyDown: { args: ["i32"], returns: "bool" },
    ImageFlipVertical: { args: ["ptr"], returns: "void" },
    // A struct by value, in or out: bun:ffi has no type for it.
    LoadShader: byValue,
    IsShaderValid: byValue,
    GetShaderLocation: byValue,
    SetShaderValue: byValue,
    BeginShaderMode: byValue,
    UnloadShader: byValue,
    LoadModel: byValue,
    IsModelValid: byValue,
    UnloadModel: byValue,
    LoadRenderTexture: byValue,
    BeginTextureMode: byValue,
    UnloadRenderTexture: byValue,
    LoadImageFromTexture: byValue,
    ExportImage: byValue,
    UnloadImage: byValue,
    ClearBackground: byValue,
    BeginMode3D: byValue,
    DrawCube: byValue,
    DrawCubeWires: byValue,
    DrawModelEx: byValue,
    DrawText: byValue,
    CheckCollisionBoxes: byValue,
  });
}

/** Every shim function takes the raylib function's address first, then pointers where C takes a struct. */
function openShim() {
  return dlopen(shimFile, {
    skyhop_LoadShader: { args: ["ptr", "ptr", "ptr", "ptr"], returns: "void" },
    skyhop_LoadModel: { args: ["ptr", "ptr", "ptr"], returns: "void" },
    skyhop_LoadRenderTexture: { args: ["ptr", "ptr", "i32", "i32"], returns: "void" },
    skyhop_LoadImageFromTexture: { args: ["ptr", "ptr", "ptr"], returns: "void" },
    skyhop_IsShaderValid: { args: ["ptr", "ptr"], returns: "bool" },
    skyhop_GetShaderLocation: { args: ["ptr", "ptr", "ptr"], returns: "i32" },
    skyhop_SetShaderValue: { args: ["ptr", "ptr", "i32", "ptr", "i32"], returns: "void" },
    skyhop_BeginShaderMode: { args: ["ptr", "ptr"], returns: "void" },
    skyhop_UnloadShader: { args: ["ptr", "ptr"], returns: "void" },
    skyhop_IsModelValid: { args: ["ptr", "ptr"], returns: "bool" },
    skyhop_UnloadModel: { args: ["ptr", "ptr"], returns: "void" },
    skyhop_BeginTextureMode: { args: ["ptr", "ptr"], returns: "void" },
    skyhop_UnloadRenderTexture: { args: ["ptr", "ptr"], returns: "void" },
    skyhop_ExportImage: { args: ["ptr", "ptr", "ptr"], returns: "bool" },
    skyhop_UnloadImage: { args: ["ptr", "ptr"], returns: "void" },
    skyhop_ClearBackground: { args: ["ptr", "ptr"], returns: "void" },
    skyhop_BeginMode3D: { args: ["ptr", "ptr"], returns: "void" },
    skyhop_DrawCube: { args: ["ptr", "ptr", "f32", "f32", "f32", "ptr"], returns: "void" },
    skyhop_DrawCubeWires: { args: ["ptr", "ptr", "f32", "f32", "f32", "ptr"], returns: "void" },
    skyhop_DrawModelEx: { args: ["ptr", "ptr", "ptr", "ptr", "f32", "ptr", "ptr"], returns: "void" },
    skyhop_DrawText: { args: ["ptr", "ptr", "i32", "i32", "i32", "ptr"], returns: "void" },
    skyhop_CheckCollisionBoxes: { args: ["ptr", "ptr", "ptr"], returns: "bool" },
  });
}

type Library = ReturnType<typeof openLibrary>["symbols"];
type Shim = ReturnType<typeof openShim>["symbols"];

export type Raylib = {
  /** raylib's own functions, called directly. */
  readonly c: Library;
  /** The by-value calls. */
  readonly shim: Shim;
  /** The address of a raylib function, to hand to the shim. */
  readonly fn: (name: keyof Library) => Pointer;
};

export type Opened = { readonly ok: true; readonly raylib: Raylib } | { readonly ok: false; readonly path: string };

/** Loads raylib at run time. A library that does not open, or lacks a function, is reported, never thrown. */
export function openRaylib(path: string): Opened {
  let c: Library;
  try {
    c = openLibrary(path).symbols;
  } catch {
    return { ok: false, path };
  }
  const shim = openShim().symbols;
  // bun:ffi puts the symbol's address on the bound function as `ptr`; its type declarations leave that out.
  const fn = (name: keyof Library): Pointer => (c[name] as unknown as { readonly ptr: Pointer }).ptr;
  return { ok: true, raylib: { c, shim, fn } };
}
