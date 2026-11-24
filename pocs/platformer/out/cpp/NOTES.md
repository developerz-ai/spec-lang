# C++ backend notes

> **Hand projection.** There is no `spec-lang build` for a game app. This folder is what one model session wrote by hand from `src/hop/*.slg`, `src/apps/cpp.slg`, `docs/game-vocabulary.md` and `docs/contract.md`, standing in for `spec-lang build --target cpp`.

- **Projected by:** Claude Fable 5.2 (`claude-fable-5-2`), one session, `As of 2026-11-24`.
- **Toolchain:** C++23, CMake 3.28 with Ninja, raylib 5.5 from `../../vendor/raylib`. Built and tested with g++ 16.0.1 (the default `c++`), g++ 13.3 (what the CI image has) and clang++ 22.1.8, all with `-Wall -Wextra -Wpedantic -Werror`.
- **Wall-clock:** about 27 minutes from first read to green gate and parity, by the session's own `date +%s` calls, two contract updates included. Not timed with anything better.

## Result

| Check | Command | Result |
|---|---|---|
| Gate | `bin/check` (`bin/build` with `-Werror`, then `ctest`, under `xvfb-run -a` when no display is set) | passes; 14 test executables, 115 tests |
| Parity | `bun scripts/parity/run.ts --only cpp` | 46/46 cases |
| Pixel agreement | `bun scripts/parity/run.ts` (all four backends built) | 4/4 `shot` lines alike across rust, bun, ruby, cpp, on one machine (Mesa llvmpipe under Xvfb) |
| The frame | `bin/skyhop cpp shot --at 150 --out <file.png> < parity/inputs/meadow-win.txt` | looked at by the projecting model: upright, HUD top left (`gems 1/3`, `lives 3`), Pip in the air left of the stone step, the crawler on it, a gem at the right edge |

## Layout

| Spec | Files |
|---|---|
| `hop.geom` | `include/hop/geom/`: `vec3.hpp`, `box.hpp`, `scalar.hpp` (`triangle`, `approach`, floored `mod`), `text.hpp`; `src/geom/text.cpp` |
| `hop.model` | `include/hop/model/`: `pad.hpp`, `rgb.hpp`, `level.hpp`, `player.hpp` |
| `hop.levels` | `include/hop/levels/`: `grid.hpp` (`q`, `at`, `slab`), `meadow.hpp`, `ridge.hpp`, `levels.hpp` (`LevelNo`, `level`) |
| `hop.physics` | `include/hop/physics/`: `constants.hpp`, `bodies.hpp`, `steer.hpp`, `slide.hpp`, `fall.hpp`, `crawl.hpp` |
| `hop.world` | `include/hop/world/`: `world.hpp` (the seven states), `taken.hpp`, `crawlers.hpp`, `lines.hpp` (`out.line`), `rules.hpp`, `view.hpp`; `src/world/`: `start.cpp`, `step.cpp`, `lose.cpp`, `summary.cpp`, `lines.cpp` |
| `hop.render` | `include/hop/render/`: `frame.hpp`, `scene.hpp`, `text.hpp`; `src/render/`: `scene.cpp`, `text.cpp` |
| `hop.screen` | `include/hop/screen/`: `screen.hpp` (`check_collision_boxes`, `read_pad`, `present`), `session.hpp` (window, assets, off-screen frame); `src/screen/`: `edge.hpp` and `edge.cpp` (every `foreign c` declaration and the section 5 conversions), `assets.cpp`, `present.cpp`, `read_pad.cpp`, `session.cpp` |
| `game` vocabulary | `include/game/cli/` and `src/cli/parser.cpp`; `include/game/script/` and `src/script/reader.cpp`; `include/game/app/` and `src/app/`: `run.cpp`, `draw.cpp`, `shot.cpp`, `play.cpp`, `replay.cpp`, `program.cpp`, `fnv.cpp`, `io.cpp`, `main.cpp` |
| `hop.laws`, `hop.tests.*`, start worlds, `@cases` | `tests/`: `laws_test`, `moves_test`, `touches_test`, `runs_test`, `edge_test`, `start_worlds_test`, `geom_cases_test`, `levels_cases_test`, `physics_cases_test`, `render_cases_test`; plus `cli_parser_test`, `script_reader_test`, `app_test`, `screen_test` |

CMake targets enforce the purity rule: `hop_sim` (geom, model, levels, physics, world, render) links nothing and has no raylib include path; `hop_screen` is the only target that links raylib, privately; `game_cli` and `game_script` link only `hop_sim`. `raylib.h` is included by `src/screen/edge.hpp`, `src/screen/assets.hpp` and `src/screen/session.cpp` and by nothing else; no public header names a raylib type.

## Tests

A 150-line harness in `tests/harness.*`, no framework, no mocks.

| Spec | Test |
|---|---|
| every `@cases` | table tests, same values; several also as `static_assert`, since the functions are `constexpr` |
| the 15 named start worlds of `hop.world` | `start_worlds_test`, same names: the world builds, unnamed states keep their declared values, the body is outside every solid |
| `hop.tests.moves` (13), `.touches` (9), `.runs` (2 and `pad_of`) | same names; `out.text` in `a_gem_is_taken_once` is the kept `Lines` of the two steps |
| `hop.laws` (8) | 400 seeded traces (splitmix64), at least 400 events each, every law after every step. Five kinds of trace (random pads; held pads; always right; a winning script with noise; a winning script cut short then random). One run reached: 76 wins, 34 losses, 6337 refused steps, 283 gems, 7 stomps, 228 mover catches; the test fails if any of those is zero |
| `touch_matches_raylib` | 4000 random box pairs through `CheckCollisionBoxes` in the linked library, coordinates from `Sub`; half built to sit within one sub-metre of a shared face on every axis, a quarter inside out (the type does not order `lo` and `hi`); fails unless both answers each cover a fifth of the pairs |
| contract section 2 | `cli_parser_test` (every usage error of the contract and `args.txt`), `script_reader_test` (every refusal, UTF-8), `app_test` (`run`, `draw`, FNV-1a test vectors, no input read before a usage error) |
| `shot` | `screen_test`: runs the built `skyhop` in its own process under the X server: the hash is the same twice, differs between worlds, the PNG is 320 by 180 and the hash is unchanged by `--out`, a missing asset and an unwritable PNG are exit 3, no display is exit 3, no key is down in a fresh window. Skipped with a printed reason when neither `DISPLAY` nor `WAYLAND_DISPLAY` is set |

## The FFI mechanism

None is needed: C++ includes `raylib.h` and calls the C functions directly. The library is linked when the backend is built (`vendor/raylib/lib/libraylib.so`), found again at run time through the run path `$ORIGIN/../../../vendor/raylib/lib`, and `SKYHOP_RAYLIB` is ignored, as contract section 3 says for a build-time-linked backend. Structs by value (`Vector3`, `Color`, `BoundingBox`, `Camera3D`, `Model`, `Shader`, `RenderTexture2D`, `Image`) are the compiler's own calling convention. No C shim, no wrapper library.

What it could not do: nothing in the contract. One thing it had to step around: `raylib.h` is a C header that is not clean under `-Wpedantic` in C++, so its include directory is a system include directory (the imported CMake target), and warnings are not reported for it.

## Libraries

None beyond the C++ standard library and raylib. Considered and declined: `fmt` (`std::to_chars` and string append cover every format here), doctest or Catch2 (the harness covers what the tests need and a download would make an offline build fail), `std::flat_map` and `std::print` (not in the g++ 13 of the CI image).

## Open choices: where the source did not determine the code

| Question | Chosen | Why |
|---|---|---|
| A flag seen twice whose second value is also bad (`--at 1 --at x`) | `error: --at given twice` | The contract lists "given twice" before the value rules. No parity case pins it. |
| The argument `--` | `error: unknown flag --` | It starts with `--`. |
| What an invalid UTF-8 line reports as its key | `error: input line N: unknown key "<U+FFFD>"` (the three bytes EF BF BD), for a line that is not UTF-8 anywhere, a comment included, checked before anything else on that line | "Invalid UTF-8 counts as an unknown key at its line" names no character; U+FFFD is what a decoding reader would print. **Other backends may differ; no parity case pins it.** |
| An unknown key outside ASCII | the whole code point is printed, not its first byte | `Text` is Unicode; `input.txt` pins `ÿ`. |
| A carriage return at the end of a line | part of the keys field: `unknown key "\r"` | Blanks are spaces and tabs only. |
| `--at` longer than any integer type | saturates at 1,000,000 | A script has at most 1,000,000 ticks, so every larger tick means "the end"; exactness is kept for every observable. |
| `SKYHOP_ASSETS` set but empty | treated as unset (`assets`) | An empty path would load `/shaders/lit.vs`. |
| Which path a shader failure names | the vertex shader's, `<assets>/shaders/lit.vs` | One error line, two files. Not reachable by a missing file, see Could not keep. |
| A frame that cannot be read back (`LoadImageFromTexture` gives no data) | `error: window: cannot read the frame`, exit 3 | The contract has no line for it; printing a hash of nothing would be a lie. Not observed. |
| `--out` that `ExportImage` cannot write | `error: <path>: cannot write`, exit 3, nothing on standard output | The contract has no line for it; standard output must be empty on every error. **Other backends may differ; no parity case pins it.** |
| `LoadRenderTexture` validity | not checked | The contract's sequence does not check it; a failed one shows as an unreadable frame. |
| Standard output cannot be written | `error: stdout: unwritable`, exit 1 | No code in the contract fits; the run itself succeeded. |
| A read error on standard input | what was read is the script | The reader then accepts or refuses it. |
| Range types at run time (`Sub`, `0..=359`, `0..=3`) | not checked | Every value is produced by the rules from constants in range; the laws and tests are the evidence, not a check per write. |
| `lose`'s guard | not tested at run time | See Optimizations. |
| `clang-format` | the sources are formatted with the POC family's `.clang-format`, not gated | Contract section 6 lists the gate as build with `-Werror` and `ctest`; a formatter check ties the gate to one clang-format version. |

## Optimizations

Each one is licensed by a fact the spec or the contract states.

| Trick | Fact that licenses it |
|---|---|
| `Int` is `int32_t` everywhere | Contract section 1: every value reached fits in 32 signed bits. |
| Every `fn` of `geom`, `levels`, `physics` and `bob`, `asset_name` is `constexpr`; the two levels are `constexpr` data in read-only memory; several `@cases` are also `static_assert` | The card: "pure with constant inputs is computed in the build". |
| A `Level` holds `std::span`s over static arrays; `state stage` copies seven words, never a list | `meadow` and `ridge` are `const`; values are immutable; `start` only ever assigns one of them. Equality stays structural. |
| `step` has no world copy and no rollback, and sends lines straight to the sink | The only failure is the guard, which runs before the first write; nothing after it can fail, so "all writes or none" holds without a transaction. |
| `lose` does not re-test its guard | `step`'s guard held, `step` calls `lose` at most once (`alive`), and nothing before the call writes `mode`. |
| `slide_x`, `slide_z` and `fall`: each `filter` + `map` + `min`/`max` chain is one pass keeping two integers; no list is built | The predicates are pure; a fold over a list in order gives the same minimum. `slide_x` and `slide_z` are one template over the axis. |
| `tops` is a lazy view; a mover's box is computed only on a tick that falls free of every solid | `mover_box` is pure, and `fall` reads `tops` only after the solids found nothing and `v < 0`. |
| `mover_shift` is `step * (triangle(t) - triangle(t - 1))` | `Int` is exact, so `home` cancels in the spec's difference of two boxes; `physics_cases_test` checks it against the spec's form. |
| `crawlers` is a vector sorted by `no` with no separate key; `patrol` rewrites entries in place; the hit loop walks the live vector | Every write in `hop.world` is `crawlers[c.no] = c` and a module owns its state, so key and `no` are one; `patrol` keeps `no`; the loop only removes the crawler it is looking at. |
| `taken` is a sorted vector | `Set[Int]` iterated by ascending key; a level has a handful of gems. |
| `View` borrows the world | A `fn` writes nothing, and the view does not outlive the call. |
| `draw` and `shot` hand the rules a dropping sink: no line is formatted | Contract: `draw` prints nothing. |
| The script stays run-length encoded; ticks are never expanded into a list of pads | A hold is the same `Pad` repeated; `step` is the only reader. |
| `replay` tests `mode` once per tick, through `step`'s own guard | A refused tick is the end of the run (game vocabulary). |
| The draw list is reserved at its final size | Two draws per block, one per mesh: known before the loop. |
| Release build: `-O3`, link-time optimisation | `@why("the game as one native binary")`. |

Measured once, on one machine (x86-64, Linux), by `time`, release build: `skyhop draw` over a script of 1,000,000 idle ticks on the meadow takes 26 ms of wall-clock, process start and input parsing included. The parity runner's median for this backend was 0.9 ms per run over 71 runs, most of which are argument and input errors. Neither is a benchmark suite. The binary is 107,200 bytes, dynamically linked to raylib and libstdc++.

## Could not keep

- **One static binary.** raylib is the POC's one shared library, so the executable is dynamic and finds it by a run path relative to itself; moving `build/skyhop` alone breaks it.
- **A second window in one process.** Opening, closing and opening a raylib window again in one process crashed (SIGSEGV) in 4 to 8 of 25 runs of an earlier version of `screen_test` on this machine (Xvfb, Mesa llvmpipe), with g++ 13 and g++ 16 builds alike, and never under gdb, so the cause is not established. `skyhop` opens one window per process and did not crash in 60 runs of `shot`; the tests now do the same (each window is its own process), and 100 runs of the new `screen_test` passed.
- **A missing shader file as an error.** With a missing assets folder raylib 5.5's `LoadShader` falls back to its default shader and `IsShaderValid` is true, so the first error the contract's checks can see is the first model: `error: asset <assets>/out/pip.glb: cannot load`. A present but wrong shader would draw with the default one, silently.
- **Checked ranges.** See Open choices.

## Line counts

`bun scripts/stats/loc.ts`, non-blank lines, `As of 2026-11-24`: spec 621 product and 247 test lines; this backend 2163 product and 1672 test lines, in 68 product files and 21 test files (`find include src -name '*.hpp' -o -name '*.cpp' | wc -l`, likewise `tests`). Every file carries a one-line provenance header, and the counts include comments.

## Limits

- One machine, one GL driver (Mesa llvmpipe under Xvfb). No GPU driver has drawn this backend's frames; the pixel hash proves the four backends make the same calls there, not that the picture is right.
- `play` was not run by a person: the loop, the keys and the 60 Hz pacing are untested beyond `read_pad` in a fresh hidden window.
- The laws are sampled on 400 seeded traces; a trace nobody generated can still break one. Stomps are rare in them (7 in one run).
- The window crash above is worked around, not understood.
- The open choices marked "other backends may differ" are not pinned by any parity case.
- The timings are single runs, not benchmarks.
