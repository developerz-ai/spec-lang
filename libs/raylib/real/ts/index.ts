// The `raylib` library's runtime for TypeScript (libs/raylib/real/ts): raylib 5.5's calls for an app, and the
// `screen` handler the `game` runtime drives. `build.sh` compiles the shim before the app is built.

import type * as rt from "../../runtime/index.ts";
import { calls } from "./calls.ts";
import { Screen } from "./screen.ts";

export const raylib: rt.Library = {
  name: "raylib",
  handlers: { screen: (program) => new Screen(program) },
  foreign: calls,
};
