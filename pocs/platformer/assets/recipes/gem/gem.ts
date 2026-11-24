// Gem, the collectible of Skyhop: an eight-sided brilliant-cut crystal, gold with a pale crown.
// Fits 0.5 x 0.5 x 0.5 m and is centred on its origin, so the game can spin and bob it in place.
// Its part is `glass`: the GLB material has alpha 0.25, which is what the lit shader reads (as
// colDiffuse.a) to turn on the shimmer. One draw call.

import { defineAsset, lathe } from "@developerz.ai/ts23d";
import { mix, paint, rgb, smooth } from "../_lib/shapes.ts";

const DEEP = rgb(0xff9f1c);
const GOLD = rgb(0xffd23f);
const PALE = rgb(0xfff6c8);

export const gem = defineAsset({
  name: "gem",
  kind: "prop",
  description: "Gem, the collectible of Skyhop",
  budget: { triangles: [200, 200, 200], draws: 1 },
  build(ctx) {
    const cut = lathe({
      profile: [
        [0, -0.25],
        [0.25, 0.06],
        [0.14, 0.25],
      ],
      segments: 8,
      color: GOLD,
      faceted: true,
      caps: true,
    });
    // Deep amber at the point, gold at the girdle, pale on the crown and table.
    const crystal = paint(cut, (p) => mix(mix(DEEP, GOLD, smooth(-0.25, 0.06, p[2])), PALE, smooth(0.06, 0.25, p[2]) * 0.8));
    ctx.part("crystal", crystal, { surface: "glass" });
  },
});
