# emit-ruby: the Ruby printer

> **State `As of 2026-11-24`.** A printer over the LIR ([decision 009](../../../docs/decisions/009-one-compiler-any-language.md)), `spec_lang_lir::Printer` for `@target(ruby)`. On the 40 programs of `src/bin/corpus` its printed checks report what the engine reports, check by check. The two POC apps build with it: `pocs/todo-app/gen/ruby` (parity 68 of 68) and `pocs/platformer/gen/ruby` (parity 46 of 46, the four `shot` lines alike with `out/ruby`). It names no program and no library.

```bash
spec-lang build <project> --target ruby --tests --out <dir>   # then <dir>/bin/test: one JSON line per check
spec-lang build <project> --app <name> --out <dir>            # then <dir>/bin/build, bin/check, exe/<bin>
```

## What it writes

| File | From |
|---|---|
| `lib/<module path>.rb` | one per module: its types, constants, states, `fn`s, rules and laws in source order, each under `# rule add (todo/board.slg:21)` and its `@why` |
| `lib/program.rb` | the program as data (`PROGRAM`): every state, rule, `fn`, law, handler with its pretend arms, and each app's manifest |
| `test/checks/<module path>.rb`, `test/checks.rb`, `bin/test` | every test, `@cases` row, property and law, under the names `spec-lang test` gives |
| `lib/spec.rb`, `lib/spec/` | the runtime, `prelude/` here, the same in every project |
| app only: `lib/libs/<library>/`, `test/libs/<library>/` | copied from `libs/<library>/real/ruby/` for each library the app uses |
| app only: `lib/app.rb`, `exe/<bin>`, `bin/build`, `bin/check`, `Gemfile` | the glue: `PROGRAM.apps`' manifest handed to the first library's `run` |

A library's `real/ruby/runtime.conf` says what the printer needs of it: `entry` (the file to require), `module`, `executable = <note>.<param>` when it can run an app, `gem = <name> <version>` per run-time gem.

## Representation

| spec-lang | Ruby |
|---|---|
| `Int` | `Integer`: exact at any size, so nothing wraps and nothing stops for width (the corpus reaches 2^127 - 1) |
| `Text`, `Bool`, `()` | a UTF-8 `String`, `true`/`false`, `nil` |
| `List`, `Set`, `Map`, tuple | `Array`; `::Set` and `Hash` built in ascending value order; `Spec::Tuple` |
| `Option[T]` | `Spec::Some` or `Spec::NONE`, never `nil` |
| record, sum | `Data.define` extended with `Spec::Record`; a module of variants (`Spec::Sum`): one frozen value, or a `Data` class when it has fields |
| declared type | a `Spec::Type` value: `check` (E-CLAIM), and what generators and libraries read |
| rule | a module method in `Spec.rule do ... end`: the world and handler state are restored on any way out but a value |
| failure, stop, blocked | exceptions `Spec::Failure`, `Spec::Stop(code)`, `Spec::Blocked(code)` |

`mod` is `Integer#%` (floored); `trim` uses the 25 code points; `lower` is `String#downcase` plus the final-sigma rule; sorting is the value order (`Spec.compare`), stable. Generation is the engine's SplitMix64 and bands (`lib/spec/testing/`), so a seed draws the engine's inputs.

## Measured

Method: `wc -l`; `time` of one `spec-lang build`, release build, one loaded machine, one run each.

| | Lines |
|---|---|
| this printer, `src/*.rs` | 3010 |
| its runtime, `prelude/` | 1846 |
| the emitter it replaces (Rust, then its embedded Ruby runtime) | 2732, 1116; it also used `emit-ts`'s typer and plan |
| `libs/cli/real/ruby` (lib, tests) | 1010, 171 |
| `libs/game/real/ruby` | 479, 93 |
| `libs/raylib/real/ruby` | 431, 79 |

`spec-lang build` of either POC app: below 0.02 s. Parity median per run on that machine: gen-ruby 41 ms against 36 ms for the hand-written todo program; 44 ms against 34 ms for the platformer.

## Open choices

| Question | Chosen |
|---|---|
| Arguments | by position up to the first parameter with a default, by keyword from there |
| Names Ruby cannot take | a trailing underscore (`end_`, `hash_`); `self` in a claim is `this`; a top-level module named like a Ruby or runtime constant gets one too |
| A written place whose key calls a rule | `E-UNSUPPORTED`; no corpus program has one |
| A check a parameter already met | not repeated on a write of that parameter to a place of the same type; a removal from a collection is not re-checked |
| The store's shape (`libs/cli`) | from the states' types: scalars, then collections; a `Map` of records whose first field has the key's type is an array of the records |
| The store's environment variable, raylib's and the assets' | `<BIN>_STORE`, `<BIN>_RAYLIB`, `<BIN>_ASSETS`, from `@bin` |

## Limits

- Agreement is of statuses on 40 programs; messages differ, and there is no evaluation budget: a loop that does not end does not stop.
- No overflow stop exists, since `Integer` has no width. A program that the engine stops with `E-RUNTIME` past 128 bits gives a value here.
- Lines are broken at lists, chains of `and`/`or` and blocks only; a long interpolated string stays long. No rubocop in the gate: `ruby -wc` on every file, the program's checks, the runtimes' minitest.
- The app starts slower than the hand-written one (the whole program and its type table load on every run); no lazy loading, no fast store reader.
- `libs/raylib`'s asset loading (one model per variant of the sum `raylib#DrawModelEx` takes, the `lit` shader) is the platformer's shape, written generically; a second game will find what it assumes.
- Timings are one run on a busy machine.
