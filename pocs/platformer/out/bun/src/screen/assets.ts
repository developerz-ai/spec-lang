// Projected by hand from load_assets: the lit shader and one model per Asset (src/hop/render.slg, docs/contract.md section 4, docs/game-vocabulary.md). Stand-in for spec-lang build output; change the spec, not this file.

import { type Asset, assetName, assets as declared } from "../render/asset.ts";
import type { Raylib } from "./raylib.ts";
import { cString, SIZE, setMaterialShaders } from "./structs.ts";

export type Assets = {
  /** `Shader`, 16 bytes. */
  readonly lit: Uint8Array;
  readonly timeLoc: number;
  /** `Model`, 120 bytes each. */
  readonly models: Readonly<Record<Asset, Uint8Array>>;
};

export type Loaded = { readonly ok: true; readonly assets: Assets } | { readonly ok: false; readonly path: string };

function unloadModels(rl: Raylib, models: readonly Uint8Array[]): void {
  for (const model of models.toReversed()) rl.shim.skyhop_UnloadModel(rl.fn("UnloadModel"), model);
}

/** After the window opens. On a failure everything already loaded is unloaded and the path is returned. */
export function loadAssets(rl: Raylib, dir: string): Loaded {
  const vs = `${dir}/shaders/lit.vs`;
  const lit = new Uint8Array(SIZE.shader);
  rl.shim.skyhop_LoadShader(rl.fn("LoadShader"), lit, cString(vs), cString(`${dir}/shaders/lit.fs`));
  if (!rl.shim.skyhop_IsShaderValid(rl.fn("IsShaderValid"), lit)) return { ok: false, path: vs };
  const timeLoc = rl.shim.skyhop_GetShaderLocation(rl.fn("GetShaderLocation"), lit, cString("uTime"));

  const loaded: Uint8Array[] = [];
  for (const asset of declared) {
    const path = `${dir}/out/${assetName(asset)}.glb`;
    const model = new Uint8Array(SIZE.model);
    rl.shim.skyhop_LoadModel(rl.fn("LoadModel"), model, cString(path));
    if (!rl.shim.skyhop_IsModelValid(rl.fn("IsModelValid"), model)) {
      rl.shim.skyhop_UnloadModel(rl.fn("UnloadModel"), model);
      unloadModels(rl, loaded);
      rl.shim.skyhop_UnloadShader(rl.fn("UnloadShader"), lit);
      return { ok: false, path };
    }
    // raylib draws a model with its materials' shader; BeginShaderMode does not reach DrawModelEx.
    setMaterialShaders(model, lit);
    loaded.push(model);
  }
  const [PipMesh, CrawlerMesh, GemMesh, FlagMesh] = loaded as [Uint8Array, Uint8Array, Uint8Array, Uint8Array];
  return { ok: true, assets: { lit, timeLoc, models: { PipMesh, CrawlerMesh, GemMesh, FlagMesh } } };
}

/** In reverse order of loading, before `CloseWindow()`. */
export function unloadAssets(rl: Raylib, assets: Assets): void {
  unloadModels(
    rl,
    declared.map((a) => assets.models[a]),
  );
  rl.shim.skyhop_UnloadShader(rl.fn("UnloadShader"), assets.lit);
}
