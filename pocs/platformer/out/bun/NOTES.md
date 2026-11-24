# out/bun: notes

> **Hand projection.** There is no `spec-lang build` for a game app. This folder is what one model session wrote by hand from `src/hop/*.slg`, `src/apps/bun.slg`, `docs/game-vocabulary.md` and `docs/contract.md`, standing in for `spec-lang build --target bun`. Paths are relative to the POC root (`pocs/platformer/`) unless they start with `out/bun/`.

- **Projected by:** Claude Fable 5.2 (`claude-fable-5-2`), one session, `As of 2026-11-24`.
- **Toolchain:** Bun 1.4.5, TypeScript 5.9.3 (strict, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`), Biome 2.3.8, the system `cc` for the shim.
- **Gate:** `out/bun/bin/check` (`bun install --frozen-lockfile`, shim, `biome check`, `tsc --noEmit`, `bun test` under `xvfb-run -a` when no display is set, `bin/build`). Passes: 172 tests, 0 failures.
- **Parity:** `bun scripts/parity/run.ts --only bun` gives 46/46. Run with `--only bun,rust`, the four `shot` hashes agree with the Rust backend's.
- **Wall-clock:** about 20 minutes from the first read to this file, measurements included.
- **Executable:** `out/bun/dist/skyhop`, one file from `bun build --compile`. The shim is embedded in it (`import ... with { type: "file" }`); Bun 1.4.5 `dlopen`s an embedded library from inside the executable, checked by copying the binary alone to another folder and running it.

## Layout

| Spec | Folder | Files |
|---|---|---|
| `hop.geom` | `src/geom/` | `vec3.ts`, `box.ts`, `scalar.ts` (triangle, approach) |
| `hop.model` | `src/model/` | `pad.ts`, `rgb.ts`, `level.ts`, `player.ts` |
| `hop.levels` | `src/levels/` | `grid.ts`, `meadow.ts`, `ridge.ts`, `level.ts` |
| `hop.physics` | `src/physics/` | `constants.ts`, `bodies.ts`, `walk.ts`, `slide.ts`, `fall.ts`, `patrol.ts` |
| `hop.world` | `src/world/` | `world.ts` (the seven states, `View`), `outcome.ts`, `start.ts`, `step.ts` (with `lose` joined), `summary.ts` |
| `hop.render` | `src/render/` | `asset.ts`, `frame.ts`, `scene.ts`, `text.ts` (`draw_text`, `frame_lines`, `dump`) |
| `hop.screen` | `src/screen/` | `raylib.ts` (the binding), `structs.ts` (C layouts), `canvas.ts` (the nine `screen.write` rules), `collision.ts`, `read-pad.ts`, `present.ts`, `assets.ts`, `window.ts`, `offscreen.ts`, `loop.ts`, `paths.ts`; `native/shim.c` |
| `game` vocabulary | `src/cli/`, `src/script/`, `src/app/` | arguments and help; the script reader; `run`, `draw`, `shot`, `play`, `execute`, the pixel hash |
| `app SkyhopBun` | `src/main.ts` | argv, env, standard input, fds 1 and 2, exit |
| `@cases` | `test/geom/`, `test/levels/`, `test/physics/`, `test/render/` | `test.each` tables |
| `hop.tests.*`, start worlds | `test/tests/`, `test/world/`, `test/support/worlds.ts` | one `test` per spec test, same names |
| `hop.laws` | `test/laws/` | fast-check, seed 20261124, 200 traces of 700 events, every law after every step |
| `hop.tests.edge` | `test/tests/edge.test.ts` | 2,000 box pairs through the real library, 1,000 of them built to touch or nearly touch |
| cli, script, app, screen | `test/cli/`, `test/script/`, `test/app/`, `test/screen/` | parser table, reader table, end to end in process, the hash against a BigInt reference, `shot` and the window through real raylib |

A rule is a pure function: `step(world, pad)` returns `{ok: true, world, lines}` or `{ok: false, refusal: "Over"}`. Nothing is changed in place and nothing throws for a refusal. `draw` and `run` never import `src/screen/`, so they cannot open the library.

## The foreign edge: what `bun:ffi` could not do

Measured on Bun 1.4.5, Linux x86-64, by asking `dlopen` for each type and by calling raylib.

- **Mechanism:** `bun:ffi` `dlopen` of raylib at run time, path from `SKYHOP_RAYLIB`, default `vendor/raylib/lib/libraylib.so` under the working directory.
- **No struct type.** `FFIType` has integers, floats, `bool`, `ptr`, `cstring`, `function`, `buffer`. Asking for `"struct"` is refused (`Unknown type struct`); an object describing fields is refused (`param must be a string (type name) or number`). So no C struct can be passed or returned by value.
- **Passing a pointer where C wants a struct is silently wrong.** `CheckCollisionBoxes` called with two pointers to correct `BoundingBox` bytes returned `false` for boxes that touch. Nothing reports the mismatch.
- **Of the 36 raylib functions this program calls, 22 need a struct by value** (18 take one, 4 return one) and go through the shim; 14 are scalars and pointers only and are called by `bun:ffi` directly.
- **The shim** is `out/bun/native/shim.c`, 22 functions, each one statement, compiled by `out/bun/bin/shim` (`cc -std=c11 -O2 -fPIC -shared -Wall -Wextra -Werror`) into `out/bun/build/libskyhop_shim.so` (16.5 kB). It links no raylib symbol: each function takes the address of the raylib function as its first argument (`bun:ffi` puts it on the bound symbol as `.ptr`), then pointers where C takes a struct. So `SKYHOP_RAYLIB` alone decides which library is called. It needs `raylib.h` at build time for the struct types, from `vendor/raylib/include` or `SKYHOP_RAYLIB_INCLUDE`.

| Shim function | Makes the call | By value |
|---|---|---|
| `skyhop_LoadShader` | `*out = LoadShader(vs, fs)` | returns `Shader` |
| `skyhop_LoadModel` | `*out = LoadModel(path)` | returns `Model` |
| `skyhop_LoadRenderTexture` | `*out = LoadRenderTexture(w, h)` | returns `RenderTexture2D` |
| `skyhop_LoadImageFromTexture` | `*out = LoadImageFromTexture(*texture)` | takes `Texture2D`, returns `Image` |
| `skyhop_IsShaderValid`, `skyhop_GetShaderLocation`, `skyhop_SetShaderValue`, `skyhop_BeginShaderMode`, `skyhop_UnloadShader` | the same name without the prefix | takes `Shader` |
| `skyhop_IsModelValid`, `skyhop_UnloadModel` | likewise | takes `Model` |
| `skyhop_BeginTextureMode`, `skyhop_UnloadRenderTexture` | likewise | takes `RenderTexture2D` |
| `skyhop_ExportImage`, `skyhop_UnloadImage` | likewise | takes `Image` |
| `skyhop_ClearBackground` | `ClearBackground(*color)` | takes `Color` |
| `skyhop_BeginMode3D` | `BeginMode3D(*camera)` | takes `Camera3D` |
| `skyhop_DrawCube`, `skyhop_DrawCubeWires` | `DrawCube(*centre, w, h, d, *color)` | takes `Vector3`, `Color` |
| `skyhop_DrawModelEx` | `DrawModelEx(*model, *at, *axis, angle, *scale, *tint)` | takes `Model`, three `Vector3`, `Color` |
| `skyhop_DrawText` | `DrawText(text, x, y, size, *color)` | takes `Color` |
| `skyhop_CheckCollisionBoxes` | `CheckCollisionBoxes(*a, *b)` | takes two `BoundingBox` |

Called directly: `SetTraceLogLevel`, `SetConfigFlags`, `InitWindow`, `IsWindowReady`, `WindowShouldClose`, `CloseWindow`, `SetTargetFPS`, `BeginDrawing`, `EndDrawing`, `EndMode3D`, `EndShaderMode`, `EndTextureMode`, `IsKeyDown`, `ImageFlipVertical`.

Other things the interface left to this code:

- **Struct layouts are written by hand as byte offsets** in `src/screen/structs.ts` (`Model.materialCount` at 68, `Model.materials` at 80, `Material` 40 bytes, `Image.width` at 8, `RenderTexture2D.texture` at 4, `Camera3D` 44 bytes). TypeScript cannot read `raylib.h`, so `native/shim.c` carries `_Static_assert`s for every one of them: a raylib with another layout fails the build, not the frame. They are LP64 layouts.
- **`materials[i].shader = lit`** is done from TypeScript: the pointer is read out of the `Model` bytes and the 16 `Shader` bytes are copied into raylib's own array through `toArrayBuffer`. No shim function, since the shim only marshals.
- **Image bytes are hashed in place**: `toArrayBuffer(img.data, 0, width * height * 4)`, no copy.
- **`.ptr` on a bound symbol is not in `bun-types` 1.3.3**; one cast in `src/screen/raylib.ts` reaches it.
- **A `Color` could be passed as a `u32`** on x86-64 and arm64 (a 4-byte struct travels in one integer register; `ColorToInt` called that way gave the right answer). Not used: it is an ABI coincidence, not a declared type.
- **Cost of the extra hop:** about 13 ns per call (1,000,000 `CheckCollisionBoxes` calls through a shim of this shape in 12.5 to 13.9 ms, one run each inside `bun` and inside a compiled executable, other agents' load present). A frame of the meadow at tick 0 makes 22 raylib calls, 20 of them through the shim.

## Libraries

| Package | Version | Kind | Replaces |
|---|---|---|---|
| `fast-check` | 4.10.2 | dev | a hand-written seeded generator for law traces, box pairs and the hash differential; shrinks a failing trace |
| `typescript`, `@types/bun`, `@biomejs/biome` | 5.9.3, 1.3.3, 2.3.8 | dev | (toolchain) |

No runtime dependency; `trustedDependencies` is `[]`. The argument parser is by hand because contract section 2 fixes its bytes.

## Open choices: where the source did not decide the code

| # | Question | Chose | Why / where |
|---|---|---|---|
| 1 | `Int` representation | JS `number` in the simulation | Contract section 1: every value fits in 32 bits; products stay below 2^31, far inside 2^53. No floating-point operation is used on them except exact integer `+ - *`, `%`, `min`, `max` |
| 2 | `--at` of any length | `BigInt` in the parser; compared with 1,000,000 as a BigInt, and only a value at or below that becomes a number | A script has at most 1,000,000 ticks, so any larger `--at` stops where 1,000,000 does. `src/app/simulate.ts` |
| 3 | Script `ticks` of any length | Decided on the digit string: leading zeros stripped, more than 6 digits refused, then `Number` | Never parses a long number. The running total saturates at 1,000,001 |
| 4 | Default raylib path | `vendor/raylib/lib/libraylib.so` under the working directory, made absolute | Mirrors the contract's default for assets. Empty `SKYHOP_RAYLIB` or `SKYHOP_ASSETS` counts as unset |
| 5 | raylib does not load (missing file, missing symbol) | `error: raylib <path>: cannot load`, exit 3 | The contract has no line for it. A backend that links raylib at build time cannot meet this case at all |
| 6 | `--out` cannot be written (`ExportImage` false) | `error: image <path>: cannot write`, exit 3, standard output empty | The contract has no line for it |
| 7 | Which path names a shader that does not load | `<assets>/shaders/lit.vs` | Two files, one error line. See the finding below: with raylib 5.5 this line is not reachable by removing the files |
| 8 | `<assets>` in the error line | As given in `SKYHOP_ASSETS`, or `assets` | "printed as given" |
| 9 | No display and no library | The library is opened first, so `raylib ...: cannot load` wins over `window: cannot open` | Both are exit 3; no case covers it |
| 10 | Clean-up when a model fails to load | Models already loaded and the shader are unloaded, then `CloseWindow()` | The contract says only exit 3 |
| 11 | Invalid UTF-8 in the script | Decoded with replacement; the error names U+FFFD | The contract fixes the kind of error, not the character printed. A byte-order mark is kept and is an unknown key too |
| 12 | A key outside the BMP | Named whole (by code point, not UTF-16 unit) | "the first character" |
| 13 | `\r` | Not white space: `1 R\r` is `unknown key "\r"`, printed raw | The contract splits on spaces and tabs only |
| 14 | `Map[Int, Crawler]` | An array kept in ascending `no`; `start` sorts once | `crawlers.values` is by crawler number; removal and patrol keep the order |
| 15 | `Set[Int]` | A JS `Set`, replaced (never changed in place) when a gem is taken | Nothing prints its order |
| 16 | Range types (`Sub`, `0..=255`, `coyote`, `yaw`, `lives`) | Not checked at run time | Contract section 1 states every reachable value fits; the laws and the start-world tests check them |
| 17 | Standard output of `run` | Collected and written once at the end; `play` writes and flushes each frame | A script is checked whole before the first tick, so nothing can fail after output starts |
| 18 | Law traces | 700 events, not 200; held pads; half of them first follow the level's winning script for a random number of ticks | See the finding below |
| 19 | `view()` | The identity: `View` has the world's seven fields | The world is an immutable value here |
| 20 | Platform | Linux, LP64. `libskyhop_shim.so` and the offsets assume it | macOS and Windows would need another library name and build line |

## Findings

- **`bun:ffi` cannot express 22 of the 36 C calls.** A `foreign c` declaration whose C signature has a struct by value needs a compiled C file on this target. A compiler for this target must ship a C compiler step, or generate the shim ahead of time per platform.
- **The card has no C layout, so the projection hard-codes byte offsets.** They are only as right as the `_Static_assert`s in the shim; a target with no shim at all would have nothing to check them against.
- **raylib 5.5 `InitWindow` crashes with no display** (a null call after GLFW fails), in plain C too, so `IsWindowReady` is never reached. The contract now refuses before the call when neither `DISPLAY` nor `WAYLAND_DISPLAY` is set. A display that is named but not there (`DISPLAY=:987`) still crashes the process; nothing here can catch it.
- **A missing shader file is not an error.** raylib 5.5 `LoadShader` falls back to its default shader, which `IsShaderValid` accepts. Only a missing model reaches `error: asset ...: cannot load` (checked with an assets folder holding the shaders and no models, and one holding nothing: both name `out/pip.glb`).
- **A 200-tick law trace from `start` cannot end a game.** The first fall takes about 120 ticks and three are needed to lose; the shortest win in the suite is 316 ticks. With the contract's minimum, `over_is_final` checks nothing and `lost_means_no_lives` never sees a lost game. Here 200 traces give 139,761 steps: 94 gems, 4 stomps, 394 lives lost, 1,016 steps on the mover, 53 games lost, 7 won, 7,242 refused steps (printed by the test).
- **Named start worlds read the declared initial values.** Contract section 1 says nothing reads them; a `@given` that sets three states takes the other four from the declarations. They live in `test/support/worlds.ts` only.
- **`starting_again_resets_everything`** is trivially true of a pure `start`: it takes no world.

## Optimizations

Each names the fact that licenses it. Times: `hyperfine -N`, this machine (Linux x86-64), other agents building at the same time; one day, indications only.

| Trick | Fact that licenses it | Measured |
|---|---|---|
| The script stays a list of holds; ticks are never expanded | The script is the tick rule's parameter, repeated | Not timed; a 1,000,000-tick script is 10 objects |
| `filter` then `map` then `min`/`max` in `slide_x`, `slide_z`, `fall` are one loop with no list | The functions are pure; `min` and `max` do not depend on order | Not timed separately |
| `mover_shift` is `step * (triangle(t) - triangle(t - 1))`; no box is built | `shifted` adds the same `home.lo` to both sides of the subtraction | Not timed |
| `bob` and the gem yaw are computed once per frame, not per gem | Both depend only on `ticks` | Not timed |
| Argument structs at the edge are written into buffers allocated once per window | Every edge rule is `@uses(screen.write)` only: no call keeps its arguments | Not timed; `DrawText` still allocates its string |
| FNV-1a 64 on two 32-bit halves, no BigInt per byte | The prime is 2^40 + 0x1b3, so each product stays below 2^53; a 300-run differential test against the BigInt definition | Not timed separately; `shot --at 150` on the meadow is 234 ± 78 ms against 272 ± 36 ms for the Rust backend, 15 runs under `xvfb-run`, dominated by the window |
| `draw` and `run` do not load `src/screen/` at all | `draw` never opens a window; only `foreign` declarations touch the library | `dlopen` is never reached |
| `--no-compile-autoload-dotenv --no-compile-autoload-bunfig`, `--bytecode`, `--minify` | The program's inputs are argv, standard input and three environment variables; it is fixed at build time | `--help` 4.4 ± 0.8 ms (100 runs); Rust 1.4 ± 1.1 ms in the same session |

Whole-program numbers, same method: `run` on the meadow winning script 15.7 ± 8.6 ms (60 runs); 1,000,000 ticks of standing still on the meadow 550 ± 104 ms, on the ridge 834 ± 109 ms (10 runs each); the Rust backend 142 ± 20 ms on the meadow. The parity runner's median per command was 6.2 ms against 0.8 ms for Rust in one run with both. The executable is 110.8 MB (the Bun runtime); the Rust one is 0.4 MB.

## Line counts

| What | Lines | Files |
|---|---|---|
| `out/bun/src/**/*.ts` | 1,730 | 55 |
| `out/bun/native/shim.c` | 50 | 1 |
| `out/bun/bin/*` | 31 | 3 |
| `out/bun/test/**/*.ts` | 1,184 | 19 |
| spec: `src/hop/*.slg` + `src/apps/bun.slg` | 721 | 9 |
| spec tests: `src/hop/tests/*.slg` | 282 | 4 |

Counted with `find out/bun/src -name '*.ts' | xargs cat | wc -l` (likewise `test`), `wc -l out/bun/native/shim.c out/bun/bin/*`, `cat src/hop/*.slg src/apps/bun.slg | wc -l`, `cat src/hop/tests/*.slg | wc -l`. All lines, blank and comment included. `bun scripts/stats/loc.ts` counts non-blank lines and gives 1,534 product and 1,038 test for this folder.

## Limits

- One machine, one day: Linux x86-64, Mesa llvmpipe under Xvfb. No arm64, macOS or Windows run; the shim build line and the LP64 offsets are Linux-only.
- `play` was started under Xvfb for three seconds and printed `0 level ridge` and `1 land`; nobody held a key. The loop and `read_pad` with a key down are untested.
- The shot hashes agree with the Rust backend's on this machine. Agreement shows the same calls were made, not that the picture is right; the frame at tick 150 of the meadow was looked at once by the projecting model (hero mid-jump, crawler on the stone step, HUD upright).
- The `bun:ffi` findings are for Bun 1.4.5. `.ptr` on a bound symbol and `dlopen` of an embedded file are behaviours observed there, not documented guarantees read from its manual.
- The timings were taken with other builds running; the spreads show it.
- A named display that cannot be reached, a raylib of another version with the same symbols but other semantics, and an unwritable standard output are not handled.
- The projection was written by a model reading the spec, not derived by a tool; a second session could choose differently in every row of the open choices.
