// Projected by hand from the result of a rule: world, out.line lines, or the guard's error (docs/contract.md section 1). Stand-in for spec-lang build output; change the spec, not this file.

import type { World } from "./world.ts";

/** `(world, args) -> (world', lines) | error`. On an error the world is unchanged and no line is printed. */
export type Outcome =
  | { readonly ok: true; readonly world: World; readonly lines: readonly string[] }
  | { readonly ok: false; readonly refusal: "Over" };
