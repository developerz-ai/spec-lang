// Projected by hand from the off-screen frame of shot: frame = screen.present, drawn into a render texture and read back (docs/game-vocabulary.md, docs/contract.md section 2 "shot"). Stand-in for spec-lang build output; change the spec, not this file.

import type { View } from "../world/world.ts";
import { present } from "./present.ts";
import { cString, imagePixels, RENDER_TEXTURE_TEXTURE, SIZE, TEXTURE_SIZE } from "./structs.ts";
import { closeWindow, FLAG_WINDOW_HIDDEN, openWindow, type ScreenError, type WindowSpec } from "./window.ts";

export const SHOT_WIDTH = 320;
export const SHOT_HEIGHT = 180;

export type Shot<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly error: ScreenError };

/**
 * Opens a hidden window, draws one frame of `view` off screen and hands its R8G8B8A8 bytes to `read`,
 * in place, before the image is unloaded. With `out`, also writes the image there as PNG.
 */
export function shoot<T>(
  spec: Pick<WindowSpec, "library" | "assets" | "display" | "title">,
  view: View,
  out: string | null,
  read: (pixels: Uint8Array) => T,
): Shot<T> {
  const opened = openWindow({ ...spec, width: SHOT_WIDTH, height: SHOT_HEIGHT, flags: FLAG_WINDOW_HIDDEN });
  if (!opened.ok) return opened;
  const { rl, canvas } = opened.window;

  const target = new Uint8Array(SIZE.renderTexture);
  rl.shim.skyhop_LoadRenderTexture(rl.fn("LoadRenderTexture"), target, SHOT_WIDTH, SHOT_HEIGHT);
  rl.shim.skyhop_BeginTextureMode(rl.fn("BeginTextureMode"), target);
  present(canvas, view);
  rl.c.EndTextureMode();
  const image = new Uint8Array(SIZE.image);
  const texture = target.subarray(RENDER_TEXTURE_TEXTURE, RENDER_TEXTURE_TEXTURE + TEXTURE_SIZE);
  rl.shim.skyhop_LoadImageFromTexture(rl.fn("LoadImageFromTexture"), image, texture);
  // Bottom row first as read from the texture; flipped in place so the top row is first, as a window shows it.
  rl.c.ImageFlipVertical(image);

  const value = read(imagePixels(image));
  const exported = out === null || rl.shim.skyhop_ExportImage(rl.fn("ExportImage"), image, cString(out));

  rl.shim.skyhop_UnloadImage(rl.fn("UnloadImage"), image);
  rl.shim.skyhop_UnloadRenderTexture(rl.fn("UnloadRenderTexture"), target);
  closeWindow(opened.window);
  return exported || out === null ? { ok: true, value } : { ok: false, error: { kind: "export", path: out } };
}
