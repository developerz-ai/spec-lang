// Projected by hand from the window of play and shot: open, load_assets, close (docs/game-vocabulary.md, docs/contract.md sections 2 and 4). Stand-in for spec-lang build output; change the spec, not this file.

import { type Assets, loadAssets, unloadAssets } from "./assets.ts";
import { type Canvas, canvas } from "./canvas.ts";
import { openRaylib, type Raylib } from "./raylib.ts";
import { cString } from "./structs.ts";

const LOG_NONE = 7;
export const FLAG_WINDOW_HIDDEN = 0x80;
export const FLAG_MSAA_4X_HINT = 0x20;

export type ScreenError =
  | { readonly kind: "library"; readonly path: string }
  | { readonly kind: "window" }
  | { readonly kind: "asset"; readonly path: string }
  | { readonly kind: "export"; readonly path: string };

export type Window = { readonly rl: Raylib; readonly assets: Assets; readonly canvas: Canvas };

export type WindowSpec = {
  readonly library: string;
  readonly assets: string;
  /** False when no display is named: the window cannot open, and raylib must not be asked to try. */
  readonly display: boolean;
  readonly width: number;
  readonly height: number;
  readonly title: string;
  readonly flags: number;
};

export type OpenedWindow =
  | { readonly ok: true; readonly window: Window }
  | { readonly ok: false; readonly error: ScreenError };

/** `SetTraceLogLevel(LOG_NONE); SetConfigFlags(flags); InitWindow(...); load_assets();` */
export function openWindow(spec: WindowSpec): OpenedWindow {
  const opened = openRaylib(spec.library);
  if (!opened.ok) return { ok: false, error: { kind: "library", path: opened.path } };
  const rl = opened.raylib;
  if (!spec.display) return { ok: false, error: { kind: "window" } };
  rl.c.SetTraceLogLevel(LOG_NONE);
  rl.c.SetConfigFlags(spec.flags);
  rl.c.InitWindow(spec.width, spec.height, cString(spec.title));
  if (!rl.c.IsWindowReady()) return { ok: false, error: { kind: "window" } };
  const loaded = loadAssets(rl, spec.assets);
  if (!loaded.ok) {
    rl.c.CloseWindow();
    return { ok: false, error: { kind: "asset", path: loaded.path } };
  }
  return { ok: true, window: { rl, assets: loaded.assets, canvas: canvas(rl, loaded.assets) } };
}

/** Unload in reverse order, then `CloseWindow()`. */
export function closeWindow(w: Window): void {
  unloadAssets(w.rl, w.assets);
  w.rl.c.CloseWindow();
}
