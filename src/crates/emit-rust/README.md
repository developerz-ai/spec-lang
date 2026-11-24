# emit-rust

> **Status `As of 2026-11-24`.** The Rust printer over the LIR ([decision 009](../../../docs/decisions/009-one-compiler-any-language.md)). `src/bin/corpus` shows `agree` in its column on all 40 programs; the todo app and the platformer build from it and pass their gates and parity suites.

`RustPrinter` implements `spec_lang_lir::Printer` for `@target(rust)`. `tests(lir)` is a Cargo project whose `bin/test` runs every check; `app(lir, name)` adds the manifest as Rust data and the runtimes of the app's libraries. It names no program and no library.

## Where things are

| File | Holds |
|---|---|
| `cx.rs` | Rust types, which fields are boxed, which checks are trivial, which declarations need the machine |
| `code.rs` | a printed expression and what must run before it; argument order |
| `expr.rs` `stmt.rs` `pat.rs` `prim.rs` | every node, statement, place and pattern; the 93 primitives |
| `decls.rs` | a module's file |
| `dty.rs` | `conform.rs` and `draw.rs`: a check and a generator per declared type |
| `machine.rs` `checks.rs` | the world, atomic steps, handlers, traces; every check as data |
| `app.rs` `project.rs` `provenance.rs` | the manifest, library runtimes, the Cargo project, `provenance.json` |
| `runtime/rt/` | the prelude, copied verbatim into each project |

## Choices

| Of | Taken |
|---|---|
| `Int` | `i128`, every operation checked: outside it stops `E-RUNTIME`, as the engine does |
| Values | a struct per record, an enum per sum, `String`, `Vec`, `BTreeSet`, `BTreeMap`; a field that would contain its own type is a `Box`. Derived `Ord` is the value order |
| `Option` | `rt::Opt`, not Rust's: the language's order is `Some(x) < none` |
| The machine | `m: &mut Machine` first, only where a declaration needs it (a state, a rule, an effect, a function value) |
| A rule | `m.atomic(\|m\| ..)`: the world and the handlers' state are cloned and put back on any halt |
| A failure's data | kept as `show` prints it, so two values that show alike are equal there |
| A handler in an app | its pretend runs there too; the library's runtime reads the result (`out.text`) after the rule commits |
| A foreign body | in `tests` it blocks; in an app it calls `crate::libs::<library>::call(symbol, data)`. The app's own checks run it: the platformer's `touch_matches_raylib` is `pass` there and `blocked` in `tests` |
| Layout | `prettyplease`, so a printer bug fails the build and not rustc; `cargo fmt --check` is not part of a printed gate |

## A library's runtime

`libs/<name>/real/rust/` is copied to `src/libs/<name>/`; `mod.rs` is the module. `runtime.conf` is all the printer reads:

| Key | Means |
|---|---|
| `entry = true` | its `main(&App)` runs an app without a shape; an app with a shape is run by the shape's library |
| `executable = bin.name` | the note and parameter that name the binary |
| `provides = x`, `requires = x` | the one that provides it is also `crate::libs::x` |
| `link = link.rs` | a build script with a `link()` function |

A runtime reads `rt::app::App` (the manifest) and `rt::data` (values and types as data), so it never meets a program's Rust types.

## Measured

Method: one Linux x86-64 machine shared with other agents, Rust 1.100, 2026-11-24. Lines are non-blank (`grep -cv '^\s*$'`). Times are single runs, or the range of five.

| What | Result |
|---|---|
| Corpus | 40 of 40 programs agree; 25 hand-made mutants of six programs give the engine's status on every check (24 of them make checks fail) |
| `todo-app/gen/rust` | `bin/check` green; parity 68 of 68, `--mix` 68 of 68, `--race 40` none lost |
| `platformer/gen/rust` | `bin/check` green; parity 46 of 46, 4 of 4 machine-dependent lines, shot hashes among them, alike with `out/rust` |
| Lines | printer 4872 and prelude 1618, against 7182 and 1162 of the emitter it replaced, which built one program; runtimes: cli 1002, game 506, raylib 611 |
| `spec-lang build` | todo 0.03 to 0.04 s, platformer 0.08 to 0.10 s |
| 1,000,000 ticks of `run` | 1.57 s printed, 0.05 s hand-written (`out/rust`) |

## Limits

- **Plain translation.** None of the old emitter's optimisation strategies are here. Arguments are cloned, each rule clones the world, the store is read whole for `stats`: the 1,000,000-tick run is about 30 times slower than the hand projection.
- **No evaluation budget.** A `while` that never ends stops `E-RUNTIME` in the engine and hangs in a printed check. Unbounded recursion overflows the stack, as it does in the engine and the LIR evaluator.
- **Lint-clean is measured, not guaranteed.** The two apps pass clippy with warnings denied and nothing allowed. Other corpus programs' check projects print code clippy has remarks on.
- A state's initial value that needs the machine, and a state whose type's claim does, are `E-UNSUPPORTED`; no program of the corpus has one.
- The `game` runtime checks the app's shape (the tick rule takes one record of `Bool` fields) when the app starts, not when it is built.
- Provenance is per item, not per expression.
- `play` is checked by nobody but a person at the keyboard.
- Two programs judged the apps; another will find what the runtimes lack.
