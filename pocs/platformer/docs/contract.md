# Projection contract

> **Design, POC-local.** There is no `spec-lang build` for a game app yet. This page is what one would derive from [`src/`](../src/) and the [`game` vocabulary](game-vocabulary.md), written by hand so that four model sessions projecting the same source produce programs that cannot be told apart from outside. Where it says **must**, the [parity suite](../parity/) checks it.

## 1. Meaning

- The world is the seven states of `hop.world`. Before any command runs, the program calls `start(level)`, so the program never reads the initial values the states declare; the spec's named start worlds (`@given` tests in `hop.world`) do, for every state they do not set.
- A rule is a function `(world, args) -> (world', lines, reply) | error`. Guards run first, in written order; on an error the world is unchanged and no line is printed. A rule called inside a rule (`lose` in `step`) joins it.
- `Int` is exact. Every value this program reaches fits in a signed 32-bit integer (coordinates are `Sub`, at most 2^20; products stay below 2^31); a backend may use any integer type at least that wide, and must not use floating point for the simulation.
- `mod` is floored: `t mod n` is in `0..<n` for `n > 0`. Only `triangle` uses it, with `t >= 0`.
- Lists iterate in written order, `Map` and `Set` by ascending key. `crawlers.values` is by crawler number.
- `out.line` lines are printed in the order the rule sent them, each followed by `\n`.

## 2. Command line

Binary `skyhop`. Exit codes: `0` success, `2` usage or input, `3` the window or an asset cannot be opened.

### Help

Printed to standard output with exit 0 for `--help` or `-h` anywhere, or to standard error with exit 2 when there are no arguments. Exactly:

```text
usage: skyhop <command> [options]

commands:
  play [--level <n>]
  run [--level <n>]
  draw [--level <n>] [--at <tick>]
  shot [--level <n>] [--at <tick>] [--out <file.png>]

play opens a window: arrows or WASD to run, space to jump, Esc to quit.
run, draw and shot read a script of controls on standard input.
```

### Parsing, in this order

1. No arguments: help to standard error, exit 2.
2. `--help` or `-h` anywhere: help to standard output, exit 0.
3. The first argument is the command. Unknown: `error: unknown command "X"`.
4. The rest, left to right; the first problem met is reported:
   - An argument starting with `--` is a flag name. Not a flag of this command: `error: unknown flag --X`.
   - A flag takes the next argument, whatever it is, as its value. None left: `error: --level needs a value`, even when the flag was seen before. Seen before, with a value: `error: --level given twice`.
   - `--level`: ASCII digits, value a `LevelNo` (1 or 2), leading zeros allowed; else `error: --level: not a LevelNo`. Default 1.
   - `--at`: ASCII digits, any value 0 or more (exact, of any length); else `error: --at: not a tick`. Default: the end of the script.
   - `--out`: any non-empty text; empty is `error: --out: not a path`.
   - Anything else: `error: unexpected argument "X"`.

Every usage error is one line on standard error, exit 2, standard output empty. `X` is printed as given.

### The input script (`run`, `draw`, `shot`)

Standard input is read whole and checked before the first tick. Lines are split on `\n`; a final line without `\n` counts. On each line, `#` and everything after it is a comment; then the line is split on runs of spaces and tabs, ignoring leading and trailing ones. An empty line is skipped. Otherwise it must be exactly two fields:

| Field | Must |
|---|---|
| ticks | ASCII digits, value 1 to 100000, leading zeros allowed |
| keys | `-` (nothing held), or one or more of `L R F B J` in any order, each at most once |

Every line is checked, in order, before the total; the first bad line gives `error: input line N: <why>`, exit 2, nothing on standard output; `N` counts from 1 and includes skipped lines. `<why>` is the first that applies: `expected <ticks> <keys>` (not two fields), `ticks must be 1 to 100000`, then the keys field read left to right, the first character that is a problem: `unknown key "c"` (not a key letter; also `-` mixed with letters) or `key "R" twice` (seen before in this field). So `RRZ` is `key "R" twice` and `RZR` is `unknown key "Z"`. A script of more than 1,000,000 ticks in total is `error: input: more than 1000000 ticks`. Invalid UTF-8 counts as an unknown key at its line.

`L R F B J` set `left right forward back jump` of the `Pad`.

### `run`

1. `start(level)`; print its lines.
2. For each tick of the script in order: if `mode` is not `Playing`, stop; else `step(pad)` and print its lines.
3. Print `summary()` and `\n`. Exit 0.

The meadow, played to the flag:

```text
0 level meadow
1 land
...
316 won
end 316 won gems 3/3 lives 3 at 4992,0,0
```

### `draw`

As `run`, printing nothing, but stop after tick `--at` (tick 0 is right after `start`); a script that ends first, or a game that ends first, stops there. Then print `dump()`: each line of `frame_lines`, each followed by `\n`. Exit 0. `draw` never opens a window or loads an asset.

### `shot`

As `draw`, then draw that frame off screen and print `shot <hash>`: the FNV-1a 64-bit hash (offset `0xcbf29ce484222325`, prime `0x100000001b3`) of the image bytes, as 16 lower-case hex digits. With `--out`, also write the image as PNG there (raylib `ExportImage`). Exactly:

```c
SetTraceLogLevel(LOG_NONE);
SetConfigFlags(FLAG_WINDOW_HIDDEN);
InitWindow(320, 180, "Skyhop");            // see "No display" below
load_assets();                             // §5
RenderTexture2D rt = LoadRenderTexture(320, 180);
BeginTextureMode(rt);
present();                                 // the spec's rule, §5
EndTextureMode();
Image img = LoadImageFromTexture(rt.texture);   // R8G8B8A8, bottom row first
ImageFlipVertical(&img);                   // top row first, as the window shows it
// hash width*height*4 bytes of img.data; --out exports this img
```

then unload everything and `CloseWindow()`. Two backends on one machine **must** print the same hash. The hash depends on the GL driver, so the suite compares backends with each other, never with a fixed value.

### `play`

```c
SetTraceLogLevel(LOG_NONE);
SetConfigFlags(FLAG_MSAA_4X_HINT);
InitWindow(960, 540, "Skyhop");            // from @window; not ready: exit 3
load_assets();
SetTargetFPS(60);
start(level);                              // print its lines
while (!WindowShouldClose()) {             // Esc or the close button
  if (mode == Playing) step(read_pad());   // print its lines, flushed each frame
  BeginDrawing(); present(); EndDrawing();
}
```

then print `summary()`, unload, `CloseWindow()`, exit 0. Not covered by the parity suite.

### No display

raylib 5.5 crashes in `InitWindow` when there is no display rather than returning. So `shot` and `play`, before `InitWindow`, refuse with `error: window: cannot open`, exit 3, when neither `DISPLAY` nor `WAYLAND_DISPLAY` is set to a non-empty value. After `InitWindow`, `IsWindowReady()` false is the same error. A display that is named but cannot be reached is not covered: raylib crashes.

### Errors

| When | Standard error | Exit |
|---|---|---|
| the window does not open | `error: window: cannot open` | 3 |
| an asset does not load (`IsModelValid`, `IsShaderValid` false) | `error: asset <path>: cannot load` | 3 |
| a rule refuses outside the cases above | not reachable: `step` is only called while `Playing` | |

Standard output is empty on every error.

## 3. Raylib, the C library

- raylib **5.5**, built by `bin/raylib` into `vendor/raylib` (`include/raylib.h`, `lib/libraylib.so`). Every backend uses this one shared library.
- Bindings are written by hand in each backend against `raylib.h`, through the language's own foreign function interface. No raylib binding package (crates, npm, gems or C++ wrappers). A small C shim is allowed where the language's interface cannot pass a C struct by value; it must be only argument marshalling, listed in `NOTES.md`.
- Finding the library: a backend that loads raylib at run time (`dlopen` and the like) takes the environment variable `SKYHOP_RAYLIB` (absolute path to `libraylib.so`) first, otherwise its own default, documented in its `NOTES.md`. A backend that links raylib when it is built links `vendor/raylib/lib` with a run path and ignores the variable. Finding the assets: `SKYHOP_ASSETS` (absolute path of the POC's `assets/` folder) wins; otherwise `assets/` relative to the working directory. The parity runner sets both.

## 4. Assets

`load_assets()`, after the window opens:

1. `lit = LoadShader("<assets>/shaders/lit.vs", "<assets>/shaders/lit.fs")`; `time_loc = GetShaderLocation(lit, "uTime")`.
2. For each `Asset` in declared order (`PipMesh CrawlerMesh GemMesh FlagMesh`): `LoadModel("<assets>/out/<asset_name>.glb")`, then set `materials[i].shader = lit` for every material of that model. raylib draws a model with its materials' shader; `BeginShaderMode` does not reach `DrawModelEx`.

Unload in reverse order before `CloseWindow()`. See [`assets/README.md`](../assets/README.md) for what the files are.

## 5. The foreign edge

`hop.screen` declares every call. A `Sub` becomes `float` as `(float)v / 256.0f`, exact for every `Sub`. A `Vec3` becomes `Vector3{x, y, z}` that way. An `Rgb` becomes `Color{r, g, b, 255}`.

| Declaration | C call |
|---|---|
| `clear_background(color)` | `ClearBackground(color)` |
| `begin_mode_3d(eye, target)` | `BeginMode3D((Camera3D){ eye, target, up = {0, 1, 0}, fovy = 45.0f, projection = CAMERA_PERSPECTIVE })` |
| `end_mode_3d()` | `EndMode3D()` |
| `begin_lit(time)` | `float t = (float)time; SetShaderValue(lit, time_loc, &t, SHADER_UNIFORM_FLOAT); BeginShaderMode(lit);` |
| `end_lit()` | `EndShaderMode()` |
| `draw_cube(box, color)` | `DrawCube(centre, w, h, d, color)`: `centre = (lo + hi) / 512.0f` per axis, `w = (hi.x - lo.x) / 256.0f`, likewise `h` (y) and `d` (z) |
| `draw_cube_wires(box, color)` | `DrawCubeWires` with the same arguments |
| `draw_model(asset, at, yaw)` | `DrawModelEx(model[asset], at, (Vector3){0, 1, 0}, (float)yaw, (Vector3){1, 1, 1}, WHITE)` |
| `draw_text(text, x, y, size, color)` | `DrawText(text, x, y, size, color)`; `text` as NUL-terminated UTF-8 (it is ASCII here) |
| `key_down(key)` | `IsKeyDown(key)` |
| `check_collision_boxes(a, b)` | `CheckCollisionBoxes((BoundingBox){a.lo, a.hi}, (BoundingBox){b.lo, b.hi})` |

Struct layouts are raylib 5.5's: `Vector3` three `float`; `Color` four `unsigned char`; `BoundingBox` two `Vector3`; `Camera3D` two `Vector3`, a `Vector3` up, `float fovy`, `int projection`; `Shader` `unsigned int id, int *locs`; `Model`, `RenderTexture2D`, `Image` as in `raylib.h`. `CheckCollisionBoxes`, `DrawCube` and `DrawModelEx` take structs **by value**.

## 6. Each backend

| | Rust | TypeScript | Ruby | C++ |
|---|---|---|---|---|
| Folder | `out/rust` | `out/bun` | `out/ruby` | `out/cpp` |
| Toolchain | stable Rust, edition 2024, `extern "C"` | Bun, TypeScript strict, `bun:ffi` | Ruby 4.0, Fiddle or the `ffi` gem | C++23, CMake, g++ or clang, `#include "raylib.h"` |
| Executable after `bin/build` | `target/release/skyhop` | `dist/skyhop` | `exe/skyhop` | `build/skyhop` |
| Gate `bin/check` | `cargo fmt --check`, `clippy -D warnings`, `cargo test`, build | `biome check`, `tsc --noEmit`, `bun test`, build | `ruby -w` on every file, minitest, build | build with `-Wall -Wextra -Wpedantic -Werror`, ctest |

Layout is production style: many small files, one concern each, mirroring the spec's modules.

| Spec | Becomes |
|---|---|
| `hop.geom`, `hop.model`, `hop.levels` | value types and pure functions; the two levels as data |
| `hop.physics`, `hop.world` | the world and the two rules; pure, no I/O, no raylib |
| `hop.render` | the frame as data and `frame_lines`; pure |
| `hop.screen` | the raylib binding and `present`, `read_pad`; the only code that touches raylib |
| `game` vocabulary | `cli/` (arguments, help, exit codes), `script/` (the input reader), `app/` (`run draw shot play`) |
| `hop.laws` | property tests: at least 200 seeded traces of 200 ticks each, from `start` of a random level then random pads; every law after every step |
| `hop.tests.*`, start worlds in `hop.world` | one native test per spec test, same names |
| `hop.tests.edge.touch_matches_raylib` | a property test through the real binding: at least 2,000 random box pairs, coordinates drawn from `Sub`, half of them built to touch or nearly touch |
| `@cases` | table tests |

Rules:

- Every source file starts with a comment naming the declarations it projects and their spec file, and saying it is a hand projection: `Projected by hand from <decls> (<spec file>). Stand-in for spec-lang build output; change the spec, not this file.`
- The simulation (`geom model levels physics world render`) never touches I/O, raylib, the clock or the environment.
- No mocks. Tests that need raylib load the real library; those that need a window run under `xvfb-run -a` and are skipped with a printed reason only when no X server can be started.
- Libraries: any well-known one a production team in that language would reach for, except a raylib binding and an argument parser (the parser is derived and must match §2 exactly). Pinned, listed in `NOTES.md` with what each replaces.
- Compiler-grade output: an optimization is allowed when a fact in the spec licenses it; `NOTES.md` names the fact.
- `NOTES.md` in the folder: every choice this contract or the spec left open and what was chosen, the FFI mechanism and anything it could not do, the projecting model, line counts with the command that counted them.

## Limits

- A real compiler would derive this page. Its gaps are found by the backends that fall into them.
- The pixel hash is only comparable on one machine and one GL driver. Two backends that agree may still share a bug the hash cannot see; the draw text (§2 `draw`) is the fixed-byte check of the frame.
- `play` is checked by nobody but a person at the keyboard.
