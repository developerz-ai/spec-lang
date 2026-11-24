// Projected by hand from where the foreign c library and the assets are found (src/hop/screen.slg, docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

import { isAbsolute, resolve } from "node:path";

type Env = Readonly<Record<string, string | undefined>>;

/** `SKYHOP_RAYLIB` first; otherwise the POC's own build, relative to the working directory. Empty counts as unset. */
export function raylibPath(env: Env): string {
  const given = env.SKYHOP_RAYLIB;
  const path = given !== undefined && given !== "" ? given : "vendor/raylib/lib/libraylib.so";
  // dlopen searches the system for a bare name; a path is always meant here.
  return isAbsolute(path) ? path : resolve(path);
}

/** `SKYHOP_ASSETS` wins; otherwise `assets` relative to the working directory. Empty counts as unset. */
export function assetsDir(env: Env): string {
  const given = env.SKYHOP_ASSETS;
  return given !== undefined && given !== "" ? given : "assets";
}

/**
 * Whether a display is named at all. raylib 5.5's `InitWindow` crashes (a null call after GLFW fails to start)
 * when there is none, so `IsWindowReady` cannot be reached to report it; this is asked first.
 */
export const displayNamed = (env: Env): boolean =>
  [env.DISPLAY, env.WAYLAND_DISPLAY].some((d) => d !== undefined && d !== "");
