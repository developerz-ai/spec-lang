# Skyhop assets

The art of the platformer POC: four models written as [ts23d](https://github.com/developerz-ai/ts23d) recipes (TypeScript compiled to GLB, deterministic) and one GLSL 330 shader pair. Every backend loads these files through raylib 5.5's own `LoadModel` and `LoadShader`; `bin/check` proves they load and render there.

**Status `As of 2026-11-24`:** `bin/check` passes on one machine (Linux x86-64, Mesa llvmpipe).

## Models

| File | What | Fits (X x Y x Z in the game, metres) | Origin | Triangles | Draw calls | Bytes |
|---|---|---|---|---|---|---|
| `out/pip.glb` | Pip, the hero: a round orange hopper with two tall ears, a cream belly and yellow feet | 0.5 x 0.75 x 0.5 | feet, bottom centre | 1688 | 1 | 61004 |
| `out/crawler.glb` | the enemy: a purple spotted beetle with club antennae and six legs | 0.75 x 0.5 x 0.75 | feet, bottom centre | 1676 | 1 | 66104 |
| `out/gem.glb` | the collectible: an eight-sided gold crystal | 0.5 x 0.5 x 0.5 | centre | 32 | 1 | 5116 |
| `out/flag.glb` | the goal: a striped pole on a stone foot, a red pennant flying toward +X | 1.0 x 3.0 x 1.0 | foot, bottom centre | 708 | 1 | 35420 |

Measured by `bin/build` (triangles and draw calls: the line it prints per asset, counted from the compiled mesh and the GLB's primitives) and `wc -c out/*.glb` (bytes). Measured bounding boxes, as raylib reports them, are in the output of `bin/view`; the largest departure from the table is Pip at 0.497 x 0.753 x 0.506.

Each GLB is LOD0 only, one mesh, one primitive, vertex colour (`COLOR_0`, normalised u16 vec4), no texture, no animation. Both characters and the pennant are original designs.

### Axes and the draw call

In the game: metres, Y up, the characters face +X. No correction is needed:

```c
Model pip = LoadModel("assets/out/pip.glb");
DrawModelEx(pip, position, (Vector3){0.0f, 1.0f, 0.0f}, 0.0f, (Vector3){1.0f, 1.0f, 1.0f}, WHITE);
```

That stands the model upright with its feet at `position.y` (the gem: its centre at `position`), facing +X, at its real size. To face -X, pass 180 degrees about the same axis.

How it gets there: a ts23d recipe is Z up with +X front, and ts23d's exporter writes recipe +X to glTF +Z and recipe +Y to glTF +X. Each recipe is authored facing +X and ends with `faceGameX`, an exact quarter turn about Z, so the GLB faces glTF +X. raylib's loader applies the GLB's root rotation to the vertices at load time.

### Colours

The recipes write display (sRGB) colour values straight into `COLOR_0` with `rgb()` in `recipes/_lib/shapes.ts`, not ts23d's linear `hex()`: raylib copies `COLOR_0` to the vertex colour with no transfer function, and the shader does its lighting on display values. A glTF viewer that treats `COLOR_0` as linear shows these models paler than the game does.

## Shader

`shaders/lit.vs` and `shaders/lit.fs`, GLSL 330, for the whole 3D pass.

```c
Shader lit = LoadShader("assets/shaders/lit.vs", "assets/shaders/lit.fs");
int timeLocation = GetShaderLocation(lit, "uTime");

// once per model: BeginShaderMode does not reach DrawModelEx
for (int i = 0; i < model.materialCount; i++) model.materials[i].shader = lit;

// every frame
float tick = (float)simTick;
SetShaderValue(lit, timeLocation, &tick, SHADER_UNIFORM_FLOAT);
BeginMode3D(camera);
BeginShaderMode(lit);   // DrawCube, DrawCubeWires, DrawSphere
// ... and DrawModelEx, which uses the shader on its materials
EndShaderMode();
EndMode3D();
```

| Name | Kind | Set by | Meaning |
|---|---|---|---|
| `vertexPosition`, `vertexTexCoord`, `vertexNormal`, `vertexColor` | attributes | raylib | the default names |
| `mvp`, `matModel`, `matNormal`, `matView` | uniforms | raylib | set for every batch and mesh draw; `matView` gives the camera position for the rim light |
| `colDiffuse` | uniform | raylib | the material colour times the tint. An alpha below 0.99 marks a glass material: drawn opaque, with the shimmer |
| `texture0` | uniform | raylib | base colour; the white default for everything here |
| `uTime` | uniform | the game | the simulation tick count as a float, an exact integer; the shader divides by 60.0 |

What it draws:

- A fixed sun from `normalize(0.4, 0.7, 0.387)`, hemisphere ambient (sky above, warm ground below), a soft rim toward the sky.
- Distance fog toward the sky colour (135, 206, 235), starting 22 m from the camera and reaching 90 % at 70 m.
- Terrain detail on flat faces whose normal is exactly a world axis, which is what `DrawCube` makes: tops get a one-metre world-space checker (±5 % brightness) and a slow ripple driven by `uTime`; sides are 8 % darker with half-metre strata. It only scales brightness, so any plain colour works.
- Gem shimmer on glass materials: a sharp highlight and a bright wave that crosses the facets about every two seconds, driven by `uTime`.
- A face-normal fallback: where the supplied normal is more than 45 degrees from the triangle's own (from screen-space derivatives of the world position), the triangle's own is used. `DrawSphere` sets no normals, so without this a sphere is lit by whatever normal the batch saw last.

Output alpha is the vertex colour's alpha times the texture's; the material alpha is not used for blending.

## Rebuild and check

```bash
bin/build             # every recipe to out/<name>.glb
bin/build --preview   # the same, then out/preview.png and out/preview-large.png drawn in raylib
bin/check             # lint, types, rebuild and compare bytes with out/, load and render in raylib twice
bin/view <out.png> <width> <height> <side|hero>   # the raylib test alone
```

Needs Bun, a C compiler, `xvfb-run`, and the POC's raylib in `../vendor/raylib` (built by the POC's `bin/raylib`).

`test/view.c` loads the shader and the four GLBs, then fails unless: the shader is valid, is not raylib's default, and has a `uTime` location; each model's bounding box is within 10 % of the table and obeys its origin rule; each model drawn alone leaves pixels with a hue, less than half of them pure white or black; the scene has at least 64 distinct colours; the frames at tick 0 and tick 90 differ. It prints the FNV-1a 64 hash of the frame's RGBA bytes, and `bin/check` runs it twice and compares.

Frame hash at 320 x 180, `side` camera, tick 90: `7301308a60aeef60`. Measured `As of 2026-11-24` on Mesa 25.2.8 llvmpipe (LLVM 20.1.2), x86-64, two runs through `bin/check`; a different driver or Mesa version may give a different hash, so the gate compares the two runs with each other, not with this value.

Pinned versions: `@developerz.ai/ts23d` 0.21.0 (`package.json`), raylib 5.5 (the POC's).

## Files

| Path | Holds |
|---|---|
| `recipes/<name>/<name>.ts` | one recipe per model |
| `recipes/_lib/shapes.ts` | shared helpers: `rgb`, `blob`, `turned`, `paint`, `faceGameX` |
| `scripts/build.ts` | compile at LOD0, run ts23d's QA, write the GLBs |
| `shaders/lit.vs`, `shaders/lit.fs` | the lit pass |
| `test/view.c` | the raylib loader and render test |
| `out/` | the built GLBs and the two preview frames |

## Limits

- Proven on one machine and one GL driver (software llvmpipe). No GPU driver, macOS or Windows has drawn these.
- The frame hash is stable run to run on that machine; nothing here shows it is equal across drivers. The face-normal fallback and fog use screen-space derivatives and interpolation, which drivers may round differently.
- "Looks good" was judged by the authoring model reading the two preview frames, not by a person.
- The models are static: no skeleton, no clips. Hopping, crawling, gem spin and bob are for the game to do with position and rotation.
- The gem's shimmer rides on its material alpha (ts23d's `glass` surface, alpha 0.25). A draw whose tint has alpha below 0.99 gets the shimmer too and is not blended; fading a model out through the tint does not work with this shader.
- The terrain detail cannot tell a platform from any other flat axis-aligned face: a cube drawn as something else gets the checker too. A cube rotated off the axes gets none.
- `DrawSphere` and `DrawCubeWires` send no normals. Spheres are rescued by the face-normal fallback and come out faceted; lines have no face and are lit by whatever normal the batch saw last.
- raylib sets `matModel` for a batch to its current matrix stack at flush time. A batch that overflows in the middle of a `DrawCube` would be flushed with a non-identity `matModel` on already-transformed vertices; this was not observed and not tested.
- `uTime` as a float is exact up to 16,777,216 ticks, about 77 hours at 60 ticks per second.
- ts23d's own contact sheets and hero renders of these recipes label the views a quarter turn off (the recipes end facing +Y) and show the colours paler; none are committed.
