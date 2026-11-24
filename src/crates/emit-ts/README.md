# emit-ts: the TypeScript printer

> **Status `As of 2026-11-24`.** `TsPrinter` implements `spec_lang_lir::Printer` for `@target(ts)` and `@target(bun)` ([decision 009](../../../docs/decisions/009-one-compiler-any-language.md)). `src/bin/corpus` reports `agree` in its column for 40 of 40 programs; the two POCs' `gen/bun` pass their own gates and parity suites. It knows TypeScript and its runtime, never a program or a library by name.

## What it prints

| File | From |
|---|---|
| `src/<module>.ts` | one spec-lang module: its types, constants, `fn`s, rules and laws, each with its `.slg` line |
| `src/machine.ts` | the world: every state, the effect handlers, save and restore |
| `src/program.ts` | the program as data (`rt.Program`): rules, functions, laws, foreign calls |
| `src/app.ts`, `src/main.ts` | an app's manifest as data (`rt.App`), and the entry point |
| `src/lib/<library>/` | copied from `libs/<library>/real/ts/`; a `build.sh` there runs before the bundler |
| `src/runtime/` | copied from `runtime/`, the same in every project |
| `test/` | the tests, the pretends, every check; `test/support/` is copied from `runtime/` |
| `bin/test` | every check, one JSON line each; `bin/check` the gate and `bin/build` the executable, for an app |

| Module | Holds |
|---|---|
| `plan.rs` | names, import aliases, which `fn`s take the world |
| `types.rs` | `LTy` as a TypeScript type, `LTy` and `DTy` as runtime types |
| `expr.rs` `stmt.rs` | the function body being printed |
| `module.rs` `machine.rs` `checks.rs` `app.rs` `project.rs` | one kind of file each |
| `format.rs` | layout: Biome's formatter as a library, so a project passes `biome check` as built |

## Representation

| spec-lang | TypeScript |
|---|---|
| `Int` | `bigint`: exact and unbounded, so `+ - *` and `<` are native and never stop |
| `Text` | `string`, counted and ordered by code point in `runtime/` |
| record, sum, `Option` | a plain object; a union told apart by `kind` |
| `Set`, `Map` | `rt.OrdSet`, `rt.OrdMap`: immutable, ascending, ordered by the element's runtime type |
| a declared type | a TypeScript type and, under the same name, an `rt.Type` value: order, show, conform, structure |
| rule | `(w: World, ...args)`, its body in `rt.atomic` |
| refusal, stop, blocked | exceptions `Failure`, `Stop`, `Blocked`; only `Failure` is handled |

## A library's runtime

`libs/<name>/real/ts/index.ts` exports `const <name>: rt.Library`: `handlers` (real ones, by name), `foreign` (calls by symbol) and `main(app, program, libraries)` when it runs apps. It reads `rt.App` and `rt.Program` and imports nothing generated. `rt.start` picks the library of the app's shape, else of its notes.

## Open choices

| Question | Chosen |
|---|---|
| `Int` width | unbounded; the engine stops at 128 bits, so a program that overflows there fails on one side only |
| A loop that does not end | 3,000,000 `while` turns per check, then `E-RUNTIME`; no limit in an app |
| The executable's name | the app's note `bin`, whatever library declares it, else the app's name |
| A foreign call's library not loaded by `use` | looked for beside the libraries that were |

## Limits

- Judged on the 40 programs and, once by hand, 120 mutants (six `sed` edits per program, two non-terminating ones of `f19` left out) against the LIR evaluator, all agreeing; no proof.
- A runaway loop whose numbers grow without bound can take minutes to reach its limit.
- `biome check` still reports what a program writes itself (`x == x`, `if true`).
- A parameter default that calls a rule, and a claim, constant or initial value that reads the world, are `E-UNSUPPORTED`.
- The world is in memory and rules are synchronous; foreign bindings are one process-wide table.
- Linux with glibc for the `cli` store's lock; `tsc` and `biome` need the project's dependencies installed.
