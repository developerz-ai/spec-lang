// The `cli` library's runtime for TypeScript (libs/cli/real/ts): what `use cli;` adds to an app.

import type * as rt from "../../runtime/index.ts";
import { main } from "./main.ts";
import { Out } from "./out.ts";

export const cli: rt.Library = {
  name: "cli",
  handlers: { out: () => new Out() },
  main,
};
