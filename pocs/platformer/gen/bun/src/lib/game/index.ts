// The `game` library's runtime for TypeScript (libs/game/real/ts): what `use game;` adds to an app.

import type * as rt from "../../runtime/index.ts";
import { main } from "./main.ts";

export const game: rt.Library = { name: "game", main };
