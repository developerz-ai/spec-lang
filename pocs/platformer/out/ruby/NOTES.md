# Ruby projection: notes

> **Hand projection.** There is no spec-lang compiler for a game app. Every file in this folder was written by a model standing in for `spec-lang build --target ruby`, from `src/`, `docs/game-vocabulary.md` and `docs/contract.md`.

| | |
|---|---|
| Projected by | Claude Opus 5.5 (`claude-opus-5-5`), one session, agent `projector` |
| Toolchain | Ruby 4.0.10; run time: the standard library, plus the `ffi` gem for `shot` and `play` only |
| Gate | `bin/check`: rubocop, `ruby -wc` on every file, `bin/build`, minitest under `-w` and `xvfb-run -a` (any warning fails) |
| State `As of 2026-11-24` | `bin/check` green, 77 tests, 0 skipped; parity 46/46, with the same pixel hashes as the Rust and Bun builds on this machine |

## Layout

| Spec | Files |
|---|---|
| `hop.geom` | `lib/skyhop/geom.rb` |
| `hop.model` | `lib/skyhop/model.rb` |
| `hop.levels` | `lib/skyhop/levels.rb` |
| `hop.physics` | `lib/skyhop/physics.rb` |
| `hop.world` | `lib/skyhop/world.rb` (states, `start`, `summary`, `view`), `lib/skyhop/world/step.rb` (`step`, `lose`) |
| `hop.render` | `lib/skyhop/render.rb` |
| `hop.screen` | `lib/skyhop/screen.rb` (`read_pad`, `check_collision_boxes`), `screen/raylib.rb` (the binding), `screen/edge.rb` (contract section 5 conversions), `screen/assets.rb`, `screen/present.rb` |
| `game` vocabulary | `lib/skyhop/cli.rb`, `cli/help.rb`, `cli/parser.rb`; `lib/skyhop/script.rb`; `lib/skyhop/app.rb` (`run`, `draw`), `app/shot.rb`, `app/play.rb`, `app/window.rb`, `app/fnv.rb` |
| `app SkyhopRuby` | `exe/skyhop` |
| `hop.laws` | `test/hop/laws_test.rb`, `test/support/laws.rb` |
| `hop.tests.moves`, `touches`, `runs` | `test/hop/moves_test.rb`, `touches_test.rb`, `runs_test.rb`: one test per spec test, same names |
| start worlds in `hop.world` | `test/support/start_worlds.rb`, `test/hop/start_worlds_test.rb` |
| `hop.tests.edge.touch_matches_raylib` | `test/screen/touch_matches_raylib_test.rb` |
| `@cases` | `test/hop/cases_test.rb` (every table), `pad_of` in `runs_test.rb` |
| argument parser, script reader, executable | `test/cli/parser_test.rb`, `test/script_test.rb`, `test/app/exe_test.rb`, `test/app/fnv_test.rb` |

## The foreign function interface

**Chosen: the `ffi` gem 1.17.4 (libffi), no C shim.** Every call in contract section 5 goes straight to `libraylib.so`.

What each option can do, as found on this machine (Ruby 4.0.10, Fiddle 1.1.8, ffi 1.17.4):

| | Fiddle 1.1.8 (default gem) | ffi 1.17.4 |
|---|---|---|
| Struct argument by value | **no.** `Fiddle::Function.new(ptr, args, ret)` takes only the scalar type codes (`TYPE_VOID VOIDP CHAR … FLOAT DOUBLE VARIADIC CONST_STRING SIZE_T … BOOL`); there is no struct code. `Fiddle::Importer#struct` builds a pointer-backed `CStruct`, which can only be passed as `void *` | **yes.** `MyStruct.by_value` in `attach_function`'s argument list; libffi builds the `FFI_TYPE_STRUCT` and applies the System V classification |
| Struct return by value | no, same reason (`LoadShader`, `LoadModel`, `LoadRenderTexture`, `LoadImageFromTexture` all return structs) | yes, `MyStruct.by_value` as return type |
| Nested struct fields | `CStruct` can nest | `layout :min, Vector3` inlines; assigning a struct to such a field copies it |
| Load without RubyGems | yes (default gem in the load path) | yes, through `bundle install --standalone`'s `setup.rb`, which only adds the gem's `lib` to `$LOAD_PATH` |
| Install | ships with Ruby | precompiled `x86_64-linux-gnu` gem with an `ffi_c.so` per Ruby ABI 3.0 to 4.0; nothing compiled here |

How far Fiddle could be pushed, by the x86-64 System V rules (reasoned, **not tried**): a struct of 16 bytes or less is passed in registers, so it could be faked with scalars. `Color` (4 bytes, class INTEGER) as one `uint32_t`; `Vector3` (12 bytes: SSE, SSE) as a `double` whose bits hold `x, y` followed by a `float` `z`; `Shader` (16 bytes: INTEGER, INTEGER) as `uint32_t` plus pointer. That trick fails for every struct over 16 bytes, which the ABI passes in memory on the stack: `BoundingBox` (24), `Camera3D` (44), `RenderTexture2D` (44), `Model` (120), and so `CheckCollisionBoxes`, `BeginMode3D`, `BeginTextureMode`, `DrawModelEx`, `IsModelValid`, `UnloadModel` cannot be called from Fiddle at all, and the four struct-returning loaders cannot be read. With Fiddle the contract's C shim would be needed for ten or more functions; with `ffi` it is needed for none. The trick is also tied to one ABI, so it is not a projection a compiler should emit.

Other things the binding does by hand:

- `materials[i].shader = lit` (contract section 4): `Material` is declared as a struct (`Shader`, pointer, `float[4]`, 40 bytes) and laid over `model[:materials] + i * Material.size`.
- `SetShaderValue`'s `const void *`: one `FFI::MemoryPointer` of one float, reused every frame.
- The ffi gem itself is a C extension. "No C shim" means no C written for this POC, not no C.

## Finding raylib and the assets

| | |
|---|---|
| raylib | `SKYHOP_RAYLIB` if set; otherwise `vendor/raylib/lib/libraylib.so` of this POC, found relative to `lib/skyhop/screen/raylib.rb` (five folders up), so the default works from any working directory |
| assets | `SKYHOP_ASSETS` if set; otherwise `assets/` relative to the working directory, as the contract says |
| ffi gem | `vendor/bundle` of this folder, installed by `bin/build` |

## Libraries

| Gem | Pinned | Used | Replaces |
|---|---|---|---|
| `ffi` | 1.17.4 | run time, `shot` and `play` only | Fiddle, which cannot pass a struct by value (above) |
| `minitest` | 6.0.0 | tests | a hand-written runner |
| `rubocop` | 1.92.3 | gate | hand review of style |
| `rubocop-minitest` | 0.41.0 | gate | hand review of assertions |

Transitive versions in `Gemfile.lock`; everything installs into `vendor/bundle` (`.bundle/config`). No argument parser and no raylib gem, as the contract requires.

## Choices the source left open

### Language to Ruby

| Question | Chosen |
|---|---|
| The seven states | one immutable `World` (`Data`); a rule returns `Outcome(world, lines)`. A refusal raises `World::Over` before a new world exists, so the world is unchanged and nothing is printed |
| `step` and the `lose` it calls | one `World::Step` object per tick holding the locals; `lose` writes into the same pending world, so it joins `step` |
| `Map[Int, Crawler]`, `Set[Int]`, records, sums | frozen `Hash`, frozen `Set`, `Data.define`; sums without data are symbols (`:Playing`, `:PipMesh`); `Draw` is three `Data` classes matched with `case … in` |
| `Option[Int]` | `nil` or the Integer |
| `Int` | Ruby `Integer`: exact, no overflow; never `Float` in the simulation. `mod` is `Integer#%`, which is floored |
| The loop variable after `lose` in `step` | `at` keeps the position before the reset, as the spec's local does; the hero's new position is only in `player`. After `lose`, `alive` is false, so `at` is not read again for the fall or the flag |
| `for c in crawlers.values` while a stomp removes one | iterates the snapshot taken at the start of the loop, as the spec's expression is evaluated once |

### Command line and script

| Question | Chosen |
|---|---|
| Argument bytes | every argument is treated as bytes (`String#b`) and printed back as given; a non-ASCII digit is not a digit |
| A value flag seen before and with no value left (`--level 1 --level`) | `needs a value`, per the contract's order and the parity case |
| Two problems on one key field | first projected as "an unknown key anywhere wins" (`RRx` gave `unknown key`). The contract then settled it the other way: left to right, the first problem character wins (`RRZ` is `key "R" twice`). **settled**, implemented |
| Invalid UTF-8 in the keys field | `unknown key "<byte>"`: each byte of an invalid sequence is its own character and is printed raw. Invalid UTF-8 in the ticks field is `ticks must be 1 to 100000`, since that check comes first. Invalid UTF-8 inside a comment is ignored |
| Bytes other than space and tab (`\r`, `\v`) | part of a field, so `1 R\r` is `unknown key "\r"` (a script with CRLF line ends is refused) |
| More than 1,000,000 ticks and a bad line later | every line is checked first; the total after. **settled** by the contract the same way |
| No display | before raylib is even loaded, `DISPLAY` and `WAYLAND_DISPLAY` both unset or empty is `error: window: cannot open`, exit 3. **settled** (contract, No display), after this backend reported the crash |
| `raylib` cannot be loaded (`SKYHOP_RAYLIB` wrong) | `error: raylib <path>: cannot load`, exit 3. Not in the contract |
| `ExportImage` returns false | `error: --out <path>: cannot write`, exit 3, nothing on standard output. Not in the contract |
| The shader does not load | reported with the vertex shader's path. In practice unreachable: see Disagreements |

### Tests

| Question | Chosen |
|---|---|
| Laws | 200 traces of 200 ticks from `start` of a random level (as asked), then 50 more from the named start worlds of `hop.world`, because no trace from `start` ends a game within 200 ticks and `over_is_final` would hold vacuously. Pads are held for spans (a new pad with chance 1/6 a tick), two in three with right held. The test fails if no trace jumps, lands, takes a gem, loses a life or is refused |
| Seed | `SKYHOP_SEED`, default 20261124; trace `i` uses `seed + i` and a failure names seed and tick |
| `touch_matches_raylib` | 2,000 pairs, seeded; odd pairs random boxes from `Sub`, even pairs built so each axis of `b` starts at `a`'s far face plus -1, 0 or 1, or overlaps `a`. Fails if fewer than 200 pairs touch |
| `out.text` in a spec test | the session collects every committed line with its newline |
| Start worlds | the states a `@given` does not set keep their declared initial values: `crawlers` and `taken` start empty, not filled from the stage |
| Window tests | `bin/check` runs minitest under `xvfb-run -a` when `DISPLAY` is unset; the two window tests skip with a printed reason only when there is still no `DISPLAY` |

## Optimizations

Method for every number: Ruby 4.0.10 called directly (not through the version-manager shim), median of 51 process runs (21 for `shot`, 7 in-process for the hash), wall clock, one run of `allbench.sh` queued under `heavy` on a 16-core Linux machine that other agents were loading (load average about 15). One machine, one run, no confidence intervals: read them as orders of size.

| Trick | Fact that licenses it | Measured |
|---|---|---|
| `--disable-gems` in the shebang; `ffi` found through the standalone load path | `run` and `draw` need only the standard library; `ffi`'s `lib` is all `shot` and `play` need | `--help`: 7.9 ms without RubyGems, 41.3 ms with |
| `--disable-yjit` in the shebang (this machine sets `RUBY_YJIT_ENABLE=1`) | a run is at most a few hundred ticks of cold code in practice (the parity scripts are 316 and 549); nothing in the spec asks for a long run | `run` of the meadow: 22.1 ms off, 73.0 ms on; `shot --at 150`: 175 ms off, 201 ms on |
| Lazy loading: `run` and `draw` never load the binding, `ffi`, the window or the hash | `hop.render.dump` is pure; only `hop.screen` touches raylib | not measured apart |
| FNV-1a kept as two 32-bit halves, no `Bignum` per byte | `h * (2^40 + 0x1b3) mod 2^64 = h * 0x1b3 + ((h mod 2^24) << 40) mod 2^64`; tested against the one-Integer definition and the published vectors | 230,400 bytes: 33.6 ms against 66.3 ms (YJIT off) |
| Crawlers iterated in the `Hash`'s insertion order, no sort | `start` inserts in list order and both levels list crawlers by ascending number (`hop.levels`); `patrol` replaces in place, `remove` keeps order. A `@given` map with keys out of order would break this; none exists | not measured |
| `Edge` makes floats as `v / 256.0` in double, narrowed to float by libffi | every `Sub` is at most 2^20, so the quotient has at most 21 significant bits: exact in a double and in a float, so the narrowing is exact and equals the contract's `(float)v / 256.0f` | the pixel hashes match Rust and Bun |
| Levels built once at load as frozen constants | `meadow` and `ridge` are `const` | not measured |

Not taken: allocation-free stepping (mutable structs, no `Data#with`). A cold `run` spends about 10 ms in 316 ticks; the rest of the 22 ms is starting Ruby (7.6 ms) and loading (3.4 ms). The laws test: 200 × 200 steps alone take 1.4 s, all 250 traces with every law 2.4 s.

The parity runner's own medians for this backend were 36 ms on a quieter machine and 91 ms at load average 22. It starts `exe/skyhop` through `env ruby`, which on this machine is a version-manager shim: 54.5 ms for `run` through the shim against 22.1 ms direct. That is the machine, not the projection.

## Disagreements and findings

Implemented as the contract says in every case.

| Where | What |
|---|---|
| No display (now settled) | raylib 5.5's `InitWindow` crashes (SIGSEGV at address 0) instead of returning, so `IsWindowReady` is never asked. Seen with a 3-line C program against `vendor/raylib` (exit 139) and from Ruby (the interpreter's `[BUG]` report). The contract now checks the environment first; a named display that cannot be reached still crashes |
| Contract section 2, errors: "`IsShaderValid` false" | when `lit.vs`/`lit.fs` are missing, raylib's `LoadShader` falls back to its default shader, which `IsShaderValid` accepts. A missing shader is not reported; the first missing model is (`error: asset …/out/pip.glb: cannot load`) |
| The PNG of `shot --out` (now settled) | was upside down: `LoadImageFromTexture` of a render texture returns the bottom row first. The contract now calls `ImageFlipVertical(&img)` before the hash and the export; done here through `ffi` with the struct's own pointer |
| `banner` in a `shot` | the labels sit at x 380 and 400, y 250, laid out for the 960 × 540 window; the shot is 320 × 180, so "level clear" and "game over" are off the image. The parity case "the banner after the flag" hashes a frame where the banner cannot be seen |

## Could not keep

| Asked | What happened |
|---|---|
| The projection header on every file | `Gemfile.lock`, `.bundle/config`, `.gitignore` and this file have none; `.rubocop.yml`, `Gemfile` and both `bin/` scripts carry it as a comment |
| "No mocks" | kept. `StringIO` stands for standard output in the `CLI.run` test; `test/app/exe_test.rb` starts the real executable |
| rubocop's branch-size limit of 20 | raised to 50: `slide_x`, `scene` and the movement half of `step` keep the shape of their spec functions |

## Line counts

| | Files | Lines | Command (run in this folder) |
|---|---|---|---|
| Product code | 22 | 1,305 | `find exe lib -type f \| sort \| xargs wc -l \| tail -1` |
| of which the C edge (`screen`, `app`) | 10 | 506 | `wc -l lib/skyhop/screen/*.rb lib/skyhop/screen.rb lib/skyhop/app/*.rb \| tail -1` |
| Tests | 15 | 911 | `find test -type f \| sort \| xargs wc -l \| tail -1` |
| Gate and config | 4 | 132 | `wc -l bin/build bin/check Gemfile .rubocop.yml \| tail -1` |
| Spec projected | 13 | 1,003 | `cat ../../src/hop/*.slg ../../src/hop/tests/*.slg ../../src/apps/ruby.slg \| wc -l` |

Physical lines, comments and blank lines included. The spec count leaves out the vocabulary card and the contract, which a compiler would also need.

## Limits

- One model, one session. Nothing here shows another session would make the same choices; the table of open choices is the evidence it would not without the contract.
- The FFI comparison is from reading Fiddle 1.1.8's type list and calling `ffi`; the register trick for Fiddle is reasoned from the ABI and was not built.
- The laws are sampled from a fixed seed, not proven. Gems are reached by few traces (single digits in 250).
- Every timing is from one run on a loaded machine.
- `play` was started under a virtual X server and printed its first lines; nobody played it.
