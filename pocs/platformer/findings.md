# Findings

> **Hand-projected, engine-checked.** One coordinator wrote [`src/`](src/), the [contract](docs/contract.md) and the [parity suite](parity/). Six model sessions worked in disjoint folders, most of them at once: the toolchain in `../../src/` (so `spec-lang test` runs a game), the art in `assets/`, and one backend each in `out/`. What follows comes from their reports, their `NOTES.md` files, the parity runs and the coordinator's own checks.

## Result

**Done.** `spec-lang test src --deny warnings`: 89 checks pass and 1 is blocked. The blocked check is `touch_matches_raylib`, which reaches `foreign c` and so runs only on a backend. All four backends pass their own gates and the 46 parity cases, and print the same pixel hash for every `shot`. Each backend translated every `@cases` row, every test by name, every law as a seeded property test, and `touch_matches_raylib` against the real library (2,000 to 4,000 box pairs). Both levels are won from a script in the spec's tests and in every backend.

## Spec errors

Places where the source, the contract or the card was wrong, silent or weaker than its `@why`.

| # | Found by | What | Status |
|---|---|---|---|
| 1 | the engine (law traces) | the private rule `lose` assumed `lives > 0` without saying so; a trace start called it directly and wrote `-1` into `0..=3` | fixed: `@guard(mode == Playing else Over)` |
| 2 | the coordinator (searching for a winning script) | the ridge's high ground was 1.5 m, and a full jump reaches 380 sub-metres, 1.48 m: the level could not be finished. No law or test could see it until a test played the level to the end | fixed: 1.25 m; `the_ridge_can_be_won` pins it |
| 3 | the engine (effects check) | `present` reads seven states through `world.view()` and declared only `screen.write`. The card requires a `pub` declaration to list its effects. It now lists eight, for one function that draws a frame | fixed; the length of that list is a finding of its own |
| 4 | Rust | the contract hashed the frame as raylib reads it back: bottom row first, so `--out` wrote the picture upside down | fixed in the contract (`ImageFlipVertical`) for all four at once |
| 5 | Rust, then Bun | raylib 5.5 crashes in `InitWindow` when there is no display; the contract's "`IsWindowReady` false, exit 3" could not happen | partly fixed: the program refuses when `DISPLAY` and `WAYLAND_DISPLAY` are both empty; a display that is named but unreachable still crashes |
| 6 | Rust, Ruby | which error wins inside the keys field (`RRZ`): two backends had chosen differently | settled: left to right, first problem; two cases added |
| 7 | Bun | the contract said nothing reads the states' declared initial values; the named start worlds do | fixed in the contract wording |
| 8 | Ruby, Bun, C++ | a missing shader file is not an error: `LoadShader` falls back to raylib's default shader and `IsShaderValid` accepts it | open: the contract's check cannot see it |
| 9 | Ruby | the HUD is laid out in window pixels (960 × 540), so the banner falls outside the 320 × 180 `shot` | open: the `draw` case checks its bytes; the card has no screen-relative layout |
| 10 | Bun, Ruby | a law trace of 200 random ticks from `start` never ends a game (a loss takes three falls of about 120 ticks; the shortest win is 316), so `over_is_final` checks nothing at the contract's minimum | open: each backend steered its traces with prefixes of the winning scripts, each in its own way |
| 11 | the engine | the card's kind rule says a `fn` has only state reads, but `read_pad` and `key_down` read the keyboard (`@uses(keys.read)`) | open card question; the engine allows world reads, never writes, on a `fn` |
| 12 | the engine | `pub fn`s in both POCs (`view`, `summary`, `dump`) read state without `@uses`; the card says every `pub` declaration writes its effects | open: the engine checks it for rules only |

Card gaps the edge made plain, none fixed here:

| Gap | Where it went instead |
|---|---|
| no `f32`, no C struct layout | the contract's table of conversions (`(float)v / 256.0f`, `Vector3`, `Color{r, g, b, 255}`) |
| no form for a resource (a mesh, a shader file) | the vocabulary loads `assets/out/<asset>.glb` by name |
| no way to say what a foreign call needs before it (a window, a GL context, a shader bound) | the contract's ordered C listing for `shot` and `play` |

## The C edge, four ways

One line of spec per C function (`= foreign c "raylib#DrawCube";`) became, in each backend:

| | Rust | Bun | Ruby | C++ |
|---|---|---|---|---|
| mechanism | `extern "C"`, `#[repr(C)]` structs, linked at build time | `bun:ffi` `dlopen` | `ffi` gem 1.17.4 | `#include "raylib.h"`, linked at build time |
| struct by value | native | **not supported**: no struct type in Bun 1.4.5 | `Struct.by_value` | native |
| C shim | none | 22 of 36 functions, one statement each | none | none |
| edge lines | 599 | 547 | 277 | 288 |
| struct layout checked by | a test per struct against `raylib.h` | `_Static_assert` in the shim | the gem's layout | the compiler |

- **A wrong call is silent.** Before its shim, Bun passed pointers where C expected `BoundingBox` values, and `CheckCollisionBoxes` returned `false` for touching boxes, with no error. `touch_matches_raylib`, the one spec test the engine cannot run, is the one that would catch it. A foreign declaration needs a differential test; the card says evidence "starts at assumed", and here that was literal.
- **Fiddle, Ruby's bundled interface, cannot pass or return a struct by value.** Structs over 16 bytes (`BoundingBox`, `Camera3D`, `Model`) cannot be faked with scalars on x86-64. Ruby needed a gem only for `shot` and `play`; `run` and `draw` start without RubyGems.
- **Pixel agreement held from the first full run**: 4/4 hashes alike across four bindings, so the four make the same calls with the same floats. Exact `float`s came from the spec: integer sub-metres over a power of two. A hash cannot tell a right picture from a wrong one; each agent looked at a frame, and the flipped one was found that way.

## Surprises

- **No backend disagreed with a single `run` or `draw` byte.** The expected bytes came from a throwaway transliteration the coordinator wrote to search for winning scripts, not from any backend. Rust and Bun reported matching them on the first build; Ruby and C++ reported no disagreement, not on which build. Integer sub-metres, a fixed tick and a written iteration order left nothing to guess in the simulation. Every disagreement that did occur was at the edges: errors, the C library, the image.
- **The laws caught none of seven planted bugs.** Method: seven one-line mutants of `src/`, each run through `spec-lang test` with default counts. At first, scenarios caught 3 (a wall snap off by half a body, a jump without a fresh press, a fast fall through a floor) and laws caught none. Six targeted scenarios then caught all 7; laws still caught none. Random traces of 40 calls from `start` leave Pip near the start: they almost never reach a gem, a crawler or a mover. In a game, a law over random input is nearly a law over standing still. Three backends independently steered their traces with the winning scripts. The language has no way to say that a law's traces should start from, or follow, a named run.
- **Ownership pushed fixtures into the rules' module.** A test may set only its own module's state, so the 15 start worlds the scenarios need live in `hop.world`, beside the rules. It is the todo POC's pattern; here it is a large part of the file.
- **Finding a winning input was outside the language.** `spec-lang test` checks and never runs a program, so searching for a script that wins the ridge (a beam search over pads) needed a transliteration in another language. A `spec-lang` command that runs a rule against generated input and keeps the best trace would have replaced it.
- **The art had its own edge cases.** ts23d's exporter writes recipe +X to glTF +Z, so each recipe ends with a quarter turn. Its colours are linear, while raylib copies vertex colours raw. raylib's `DrawSphere` and `DrawCubeWires` send no normals. `BeginShaderMode` does not reach `DrawModelEx`. With one uniform, the shader tells a gem by its material alpha. Each is in [assets/README.md](assets/README.md).
- **A second raylib window in one process crashed now and then** (C++ tests, 4 to 8 of 25 runs, cause not found). Every backend opens one window per process.

## Six agents, mostly in parallel

| Agent | Model | Folder | Run time |
|---|---|---|---|
| art | Fable 5.2 | `assets/` | 10 min |
| toolchain | Opus 5.5 | `../../src/` | 19 min |
| Rust | Opus 5.5 | `out/rust/` | 15 min |
| Bun | Fable 5.2 | `out/bun/` | 18 min |
| Ruby | Opus 5.5 | `out/ruby/` | 16 min |
| C++ | Fable 5.2 | `out/cpp/` | 28 min |

Method: the harness's recorded duration of each agent, including resumes. The coordinator's own work (spec, contract, suite, script search) is not in the table and was not timed.

- **No conflicts.** Each agent wrote only its folder. The spec, contract and suite were the only shared files, and only the coordinator edited them.
- **Two changes reached agents in flight.** (1) Six tests and four start worlds were added to the spec. (2) The contract changed for findings 4 to 6. Each was one message to four agents; the Rust agent, already finished, applied change (2) in 33 s.
- **No agent asked another anything.** Every question went into a `NOTES.md` or a report and was settled once in the contract. The run times in the table add up to about 106 minutes; the team's wall time was not measured, and the art agent started before the spec was finished, so the six did not all overlap.

## Numbers

Method: one machine (AMD Ryzen 7 5700X3D, 16 threads, Linux 7.0, Mesa 25.2.8 llvmpipe under Xvfb), 2026-11-24, warm caches, load average about 13 from other builds; one run unless said.

| Loop | Covers | Time |
|---|---|---|
| `spec-lang test src` | parse, check, lint, 90 checks: 42 `@cases` rows, 39 tests, 1 property (blocked), 8 laws × 200 traces × 40 steps | 2.24 s ± 0.05 s (hyperfine, 5 runs; 28.8 s of CPU) |
| the same, `--jobs 1` | | 24.9 s ± 5.9 s (5 runs) |
| `out/rust/bin/check`, nothing changed | fmt, clippy, 78 tests, build | 1.8 s |
| `out/cpp/bin/check` | build, 115 tests under Xvfb | 3.6 s |
| `out/ruby/bin/check` | `ruby -w`, 77 tests | 8.8 s |
| `out/bun/bin/check` | biome, tsc, 172 tests, compile | 10.4 s |
| `assets/bin/check` | rebuild four GLBs, compare bytes, load and render in raylib twice | 5.0 s |
| `bin/raylib`, once | download, verify, build raylib 5.5 | not timed |

On one thread the eight laws take 2 to 3 s each, and the other checks take about 70 ms together.

Read honestly, the spec loop is not faster than a warm native gate here; the laws dominate it, and their traces barely move (see Surprises). What differs is what each loop needs. `spec-lang test` needs one binary and 7 MB of memory on one thread (14 MB on sixteen, `/usr/bin/time`): no compiler, no GL driver, no display, no raylib build, no package manager. Every native gate needs its toolchain, the C library built from source with X11 and GL headers, a virtual display, and in two languages an FFI layer to be right about struct layouts. That cost is paid before the first test of the game's logic runs, on every machine and every CI job. The game's logic was finished and checked before any of it existed.

## Verdict

One source of about 870 lines fixed a game's meaning tightly enough that four sessions, blind to each other, wrote simulations that agreed byte for byte with a fifth, written by the spec's own author, and four bindings to a C library that agreed pixel for pixel. The simulation is where spec-lang did best: integer units, a fixed tick, guards and written order left nothing to choose. The C edge is where it did least. The card's `foreign c` is one line, but the facts that line needs were written in the contract, not the spec: struct layouts, `float` conversion, call order, resources, what to do without a display. The backends spent 277 to 599 lines each on it, and one language could not do it without a C shim. Against the idea: laws, the form meant to replace hand-written tests, found nothing in a game until the inputs were steered by hand, and the hardest questions of the run (can this level be won? what does a winning input look like?) needed a program run, which the toolchain deliberately does not do. A real game would also spend most of its code in the renderer, which here was the smallest part.

## Limits

- Models wrote every file under `out/`, and each backend's tests were translated by the session that wrote its code.
- The expected bytes of `run` and `draw` came from one transliteration; agreement of five implementations is strong evidence, not proof.
- Six sessions, two models, one run each. The agent times include waiting on a shared, loaded machine.
- Two small levels. Physics without slopes, rotation or more than one moving body at a time does not test what a larger game would.
- `play` is unplayed by a person, and the pixel hash is comparable only on one machine and driver.
