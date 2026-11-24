// Projected by hand from the screen errors of play and shot as lines on standard error (docs/game-vocabulary.md, docs/contract.md section 2). Stand-in for spec-lang build output; change the spec, not this file.

import type { ScreenError } from "../screen/window.ts";
import { type Result, screenError } from "./result.ts";

export function screenFailure(e: ScreenError): Result {
  switch (e.kind) {
    case "window":
      return screenError("window: cannot open");
    case "asset":
      return screenError(`asset ${e.path}: cannot load`);
    case "library":
      return screenError(`raylib ${e.path}: cannot load`);
    case "export":
      return screenError(`image ${e.path}: cannot write`);
  }
}
