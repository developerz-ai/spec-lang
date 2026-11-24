# Rust projection: notes

> **Hand projection.** There is no `spec-lang build`. This crate is what one model session wrote by hand from `src/hop/*.slg`, `src/apps/rust.slg`, `docs/game-vocabulary.md` and `docs/contract.md`, standing in for `spec-lang build --target rust`.

- **Projected by:** Claude Opus 5.5 (`claude-opus-5-5`), one session, `As of 2026-11-24`.
- **Toolchain:** `rustc 1.100.0`, edition 2024, raylib 5.5 from `../../vendor/raylib`.
- **Wall-clock:** about 14 minutes from first read to green gate and parity, by the session's own `date +%s` calls. Not timed with anything better.

## Result

| Check | Command | Result |
|---|---|---|
| Gate | `bin/check` (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `xvfb-run -a cargo test`, `bin/build`) | passes; 78 tests |
| Parity | `bun scripts/parity/run.ts --only rust` | 46/46 cases |

## Layout

| Spec | Here |
|---|---|
| `hop.geom` | `src/geom.rs` |
| `hop.model` | `src/model.rs` |
| `hop.levels` | `src/levels.rs` (both levels as `const` data) |
| `hop.physics` | `src/physics.rs` |
| `hop.world` | `src/world/`: `mod.rs` (the seven states), `start.rs`, `step.rs`, `lose.rs`, `summary.rs`, `view.rs`, `out.rs` (`out.line`) |
| `hop.render` | `src/render.rs` |
| `hop.screen` | `src/screen/`: `ffi.rs` (the binding), `convert.rs` (section 5 conversions), `present.rs`, `keys.rs` (`read_pad`), `assets.rs` (section 4), `window.rs` (the `shot` and `play` windows, the read-back) |
| `game` vocabulary | `src/cli/` (help, parser, exit codes), `src/script/` (the input reader), `src/app/` (`run draw shot play`, FNV-1a) |
| `app SkyhopRust` | `src/main.rs`: the process edge only |
| `hop.laws` | `tests/laws.rs` |
| `hop.tests.moves`, `.touches`, `.runs`, `.edge` | `tests/moves.rs`, `tests/touches.rs`, `tests/runs.rs`, `tests/edge.rs`, same test names |
| start worlds in `hop.world` | `tests/support/mod.rs` (builders), `tests/world_given.rs` (one test each, same names) |
| `@cases` | `tests/cases.rs` |
| the rest | `tests/cli_parse.rs`, `tests/script_read.rs`, `tests/app_commands.rs` (in process), `tests/app_shot.rs` (the built binary in an X display) |

Only `src/screen/` contains `unsafe` (`unsafe_code = "deny"` for the crate, allowed in that module alone). The simulation modules import nothing from `std::io`, `std::env`, `std::fs` or `std::time`.

## FFI

- **Mechanism:** hand-written `unsafe extern "C"` declarations and `#[repr(C)]` copies of `Vector3`, `Color`, `BoundingBox`, `Camera3D`, `Shader`, `Texture`, `RenderTexture`, `Image`, `Material`, `Matrix`, `Model` in `src/screen/ffi.rs`. No crate, no bindgen, no C shim: Rust passes and returns C structs by value natively. A unit test checks every struct size against `raylib.h` on x86-64 (`Model` 120 bytes).
- **Linking:** `build.rs` adds `../../vendor/raylib/lib` to the search path, links `-lraylib` and sets an absolute `-rpath` to that folder. `SKYHOP_RAYLIB` is not read (contract section 3). Moving the checkout means rebuilding.
- **Could not do:** raylib 5.5's `InitWindow` does not stop when GLFW fails to start: it calls `rlglInit` with no GL loaded and segfaults (observed: `rlLoadTexture` jumps to address 0 with `DISPLAY` unset). So the contract's "not ready: `IsWindowReady` false, exit 3" cannot be reached through raylib. The binary refuses with `error: window: cannot open`, exit 3, before calling `InitWindow` when neither `DISPLAY` nor `WAYLAND_DISPLAY` is set. A display that is named but unreachable still crashes inside raylib. GLFW's own functions are not exported from `libraylib.so`, so they cannot be used to probe.

## Open choices

| # | Left open | Chosen | Why |
|---|---|---|---|
| 1 | Integer width | `i32` for every simulation value | contract section 1 bounds every value below 2^31 |
| 2 | `--at` beyond `u64` | saturates at `u64::MAX` | a script has at most 10^6 ticks, so every larger value stops at the end; `--level` saturates too, staying outside `1..=2` |
| 3 | `--x` with a value missing *and* given before (`--level 1 --level`) | `needs a value` first, then `given twice`, then the value's own check | the parity case fixes the first; the bullet order fixes the rest |
| 4 | Invalid UTF-8 in a script | the line is read as bytes; comment stripping, splitting and ticks work on bytes; in the keys field an invalid sequence is an unknown key printed as U+FFFD | the contract says "an unknown key at its line" and gives no text for the character. Invalid bytes in a comment are ignored; in the ticks field they are a ticks error |
| 5 | Bad line versus more than 10^6 ticks | every line is checked first, then the total | "standard input is read whole and checked"; the total is a property of the whole |
| 6 | Standard input cannot be read | `error: input: cannot read`, exit 2 | invented text; "usage or input" is exit 2 |
| 7 | Standard output cannot be written | exit 1, nothing more | the contract has no code for it; dropping it silently would hide a failure |
| 8 | The shader fails to load | `error: asset <assets>/shaders/lit.vs: cannot load`, exit 3; failure is `IsShaderValid` false **or** the id equal to raylib's default shader | raylib returns its default shader, which is valid, when a file is missing; the asset test (`assets/test/view.c`) checks the same. The path printed is the vertex shader's, a guess |
| 9 | `ExportImage` fails | `error: <path>: cannot write`, exit 3, nothing on standard output | invented; the PNG is written before `shot <hash>` is printed so a failure leaves standard output empty |
| 10 | Unloading | each model's materials get the default shader id back before `UnloadModel`, then the lit shader is unloaded once | raylib's `UnloadMaterial` unloads any non-default shader, so four models sharing `lit` would free it four times |
| 11 | Empty `SKYHOP_ASSETS` | treated as unset (`assets/`) | an empty path names nothing |
| 12 | `View.crawlers` (`List[Crawler]`) | a borrow of the map, iterated by key | `crawlers.values` is by crawler number; a `fn` writes nothing, so a borrow is the same list |
| 13 | Law traces | 240 traces, at least 200 events each, from `start` of a random level; random pads held 1 to 40 ticks; a `start` with chance 1/100 per event; one trace in three first plays a fixed meadow script (to the flag, or off the first ledge until `Lost`) | so `over_is_final` and `lost_means_no_lives` are not checked only vacuously; the test asserts a won game, a lost game, a refused step and a taken gem were all reached |
| 14 | `touch_matches_raylib` generation | 4000 pairs; even pairs share a face, or miss or overlap by one sub-metre, on a random axis | the contract asks for 2000, half near |
| 15 | `out.text` in `a_gem_is_taken_once` | one `Lines` sink for the test, appended by both steps | the test reads every line its steps sent |

## Optimizations

| Trick | Fact that licenses it |
|---|---|
| Both levels are `const` data with `&'static` slices; `start` copies no list | `pub const meadow`, `pub const ridge` |
| `slide_x`, `slide_z`, `fall` fold the extreme of the hits while scanning; no hit list is built | the filters are pure and only `min`/`max` (and `size > 0`) of them are read |
| `fall` keeps the highest catching top and the lowest number at that height in one pass | `catching.map(hi.y).max()` then `filter(hi.y == top).map(no).min()` |
| Mover tops are built on the stack (up to 8 movers; a heap list beyond) | `tops` is used once, by `fall`, in the same tick |
| `draw` and `shot` never format a line | `out.line` has no effect but the line (game vocabulary), and those commands print none |
| `run` gathers its lines and writes once | lines are only printed, in order; nothing reads standard output while the script runs |
| `View` borrows the world | `view` is a `fn`: it writes nothing |
| The frame's draw list is allocated once at its final size | the counts of solids, movers, gems and crawlers are known before the loop |
| `Rgb` fields are `u8` | `r: 0..=255` and likewise |
| `triangle` uses `rem_euclid` | `mod` is floored and the divisor `2n` is positive |
| `uTime` and every `Sub` cross the edge as `f32` without rounding | `Sub` is at most 2^20 and ticks at most 10^6, both below 2^24 |
| The help text is a constant | the app's commands and flags are fixed when it is built |
| Release profile `opt-level = 3`, fat LTO, one codegen unit, `panic = "abort"` | `unwrap`, `expect` and `panic` are denied outside tests |

## Libraries

None. `[dependencies]` is empty.

| Would have been | Why not |
|---|---|
| `raylib`, `raylib-sys` | excluded by the contract |
| `clap` | excluded by the contract |
| `proptest`, `rand` | a 12-line SplitMix64 in `tests/support/mod.rs` seeds every property and law |

## Line counts

Counted `As of 2026-11-24` with `wc -l`, blank lines and comments included.

| What | Lines | Command |
|---|---|---|
| Product code | 2506 | `wc -l $(find src -name '*.rs') \| tail -1` |
| of which the raylib edge | 662 | `wc -l src/screen/*.rs \| tail -1` |
| Tests | 1524 | `wc -l $(find tests -name '*.rs') \| tail -1` |
| Build files | 88 | `wc -l build.rs bin/build bin/check Cargo.toml clippy.toml \| tail -1` |

## Disagreements and findings

No parity case disagreed with the spec as projected here; all 45 passed on the first build; the 46th, added later for finding 3, passes after the change.

1. **The `shot` PNG was upside down.** `LoadImageFromTexture` on a render texture returns rows bottom-up. Reported; the contract now flips with `ImageFlipVertical` before hashing and export, and this backend does.
2. **"Window not ready: exit 3" was not reachable through raylib 5.5** (see FFI). Reported; the contract now has the "No display" pre-check this backend already made.
3. **Key-field errors.** This backend first reported any unknown key before any repeat (`RRx` gave `unknown key "x"`). The contract now reads the field left to right, first problem wins (`RRZ` is `key "R" twice`); changed to match.

## Limits

- One session, one machine, Linux x86-64, Mesa llvmpipe under Xvfb. `play` was not run by a person.
- The law and edge tests sample seeded inputs; they are evidence, not proof.
- Struct layouts are checked by size only, on one 64-bit target.
- The rpath is absolute: the built binary finds raylib only while the checkout stays where it was built.
