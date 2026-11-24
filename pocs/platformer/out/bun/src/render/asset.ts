// Projected by hand from Asset, asset_name (src/hop/render.slg). Stand-in for spec-lang build output; change the spec, not this file.

/** Each mesh is one model file the game loads at start. In declared order. */
export const assets = ["PipMesh", "CrawlerMesh", "GemMesh", "FlagMesh"] as const;

export type Asset = (typeof assets)[number];

const names: Record<Asset, string> = { PipMesh: "pip", CrawlerMesh: "crawler", GemMesh: "gem", FlagMesh: "flag" };

export const assetName = (a: Asset): string => names[a];
