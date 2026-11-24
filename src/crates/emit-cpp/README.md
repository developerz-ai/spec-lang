# emit-cpp

> **Status `As of 2026-11-24`.** The C++23 printer of [decision 009](../../../docs/decisions/009-one-compiler-any-language.md): `spec_lang_lir::Printer` for target `cpp`. On the 40 programs of `src/bin/corpus` its printed checks report what `spec-lang test` reports; the two POCs' `gen/cpp` pass their gates and parity suites. It names no program and no library.

## What it prints

```text
Lir ──decls.rs────> include/<module>.hpp + src/<module>.cpp   types, constants, fns, rules; each under `// rule a.b (file.slg:line)`
    ──machine.rs──> program/machine    World (every state), one class per handler and its pretend, Machine::atomic
                    program/foreign    one function per `foreign` symbol: blocked for tests, bound to lib::<library> for an app
    ──checks.rs───> tests/checks.cpp   every check, the generators of its properties and laws
    ──app.rs──────> program/data       every record and sum as rt::Data and back
                    program/manifest   the app as rt::meta::App: notes, rules, parameters, triggers, states, laws of one world
                    lib/<library>/     copied from libs/<library>/real/cpp/, each with its runtime.cmake
    + runtime/ (verbatim), CMakeLists.txt, bin/, .clang-format; every C++ file through clang-format once
```

| File | Does |
|---|---|
| `src/emit/` | expressions, statements, patterns, `conform`, the 93 primitives; `Fx` is one body being printed |
| `src/facts.rs` | what an expression can do (fail, stop, read or write state), so siblings keep left-to-right order where C++ leaves it open |
| `src/decls.rs` | a module's header in an order C++ accepts; a type that holds itself keeps that field in `rt::Box` |
| `runtime/include/rt` | `Int`, `Text` by code point, `Option`, ordered `Set` and `Map` (`std::set`, `std::map` over `<=>`), `show`, failures, traces |
| `runtime/tests/rt` | the test runner and the engine's generator (SplitMix64, FNV-1a seeds) |

**`Int` is a checked 128-bit integer** (`__int128` with `__builtin_*_overflow`): an exact result outside -2^127 .. 2^127 - 1 stops `E-RUNTIME`, as the engine's `i128` does. It needs GCC or Clang.

A language failure is a C++ exception (`rt::Failure`); a rule is `ctx.atomic([&] { ... })`, which copies the world and the handlers and puts them back on any exception.

## Run it

```bash
target/release/spec-lang build ../pocs/todo-app/src --target cpp --tests --out <dir> && <dir>/bin/test
target/release/spec-lang build ../pocs/platformer/src --app SkyhopCpp --out <dir> && <dir>/bin/check
SPEC_LANG=$PWD/target/release/spec-lang bin/corpus     # the judge
cargo test -p spec-lang-emit-cpp                        # what is printed (needs clang-format)
```

## Measured

Method: `wc -l` over the files named; build time is the wall time of `spec-lang build --app` (release binary, three runs, the middle one), which includes one `clang-format` process per C++ file. Caveat: one machine, 16 threads, other builds running.

| What | Lines |
|---|---|
| this printer, `src/` | 5,287 |
| the todo-shaped emitter it replaces, `src/` | 5,431 |
| runtime prelude (`runtime/include`, `runtime/src`) | 1,932 |
| test runner (`runtime/tests`) | 621 |
| `libs/cli/real/cpp` | 1,435, and 155 of tests |
| `libs/game/real/cpp` | 693, and 91 of tests |
| `libs/raylib/real/cpp` | 426, and 50 of tests |

| Program | `spec-lang build --app` |
|---|---|
| `pocs/todo-app/src`, `TodoCpp` | 0.20 s |
| `pocs/platformer/src`, `SkyhopCpp` | 0.85 s |

The corpus run (`heavy bin/corpus`, all four printers and both evaluators, 40 programs) took 123.5 s on that machine; the C++ column agreed on 40 of 40.

## Open choices

| Question | Chosen | Where |
|---|---|---|
| The evaluation budget | only `while` loops count (20,000,000 turns), not every node; recursion without end overflows the stack | `runtime/src/rt/value.cpp`, `tick` |
| A rule that stops (`E-CLAIM`, `E-RUNTIME`) in a built app | `error: <code>: <message>`, exit 1 | `libs/cli/real/cpp/run.cpp`, `libs/game/real/cpp/app.cpp` |
| A number argument past 128 bits | not a value of its type (a usage error) | `libs/cli/real/cpp/args.cpp`, `parse_int` |
| `start`'s parameter when its flag is absent and it has no default | the first of 1 and 0 its type admits | `libs/game/real/cpp/args.cpp`, `start_default` |
| Which files a `DrawModelEx` model loads | one per variant of the model's sum, named by the program's public fn from that sum to Text | `libs/raylib/real/cpp/binding.cpp`, `model_names` |
| The store's laws | a stored world must meet every law whose one parameter is the trace | `src/app.rs`, `manifest` |

## Limits

- Agreement with the engine was judged on the corpus only; statuses agree, messages do not.
- `Text.lower` uses tables taken from the Rust standard library of the toolchain that printed the project, so it lowers as `spec-lang test` does and no better.
- A lambda that is kept (returned, stored) and reaches a state is printed only inside a body whose machine is named `ctx`; another shape is `E-UNSUPPORTED`.
- A reply pattern over rules with different result types (`Ok(v)` binding `v`) is `E-UNSUPPORTED`; no corpus program writes one.
- The printed checks run on one thread. The generated project needs CMake 3.28, GCC or Clang with C++23, and `clang-format` for its gate; the printer itself needs `clang-format` on the path and fails with a load error without it.
- An app's parameters and results must be plain data to cross to a library (`rt::Data`); a rule taking a function is refused in an app.
- The raylib runtime links raylib 5.5 from `RAYLIB_ROOT` (default `../../vendor/raylib` from the project); the shader's file names and the asset folder layout are the platformer's, written into the runtime because the card has no form for a resource.
