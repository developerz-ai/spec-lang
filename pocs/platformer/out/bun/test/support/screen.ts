// Projected by hand from where the tests find the POC's raylib and assets (docs/contract.md section 3). Stand-in for spec-lang build output; change the spec, not this file.

import { resolve } from "node:path";
import { displayNamed } from "../../src/screen/paths.ts";

const poc = resolve(import.meta.dir, "../../../..");

/** `SKYHOP_RAYLIB` and `SKYHOP_ASSETS` when set, as the program reads them; otherwise the POC's own folders. */
export const library = process.env.SKYHOP_RAYLIB || resolve(poc, "vendor/raylib/lib/libraylib.so");
export const assets = process.env.SKYHOP_ASSETS || resolve(poc, "assets");

export const display = displayNamed(process.env);

/** Printed once by each test file that needs a window and has no X server to open it on. */
export function skipWithoutDisplay(what: string): void {
  if (!display) console.log(`skipped: ${what} needs an X server (run under xvfb-run -a); DISPLAY is not set`);
}
