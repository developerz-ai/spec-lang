# emit-ts: the TypeScript-on-Bun backend

> **State `As of 2026-11-24`.** Runs for one program: the todo POC (`pocs/todo-app/src`). `emit` turns its `@target(bun)` app into a Bun project whose own gate passes and which reproduces the POC's 68 parity transcripts. Any form outside the subset below is an `E-UNSUPPORTED` diagnostic, never a guess.

`pub fn emit(program: &ir::Program, app: &ir::AppDecl) -> Result<Vec<OutFile>, EmitError>`. Same input, same bytes. `EmitError` is a boxed `spec_lang_diag::Diag`: code, span, declaration path, expected and found where they apply, and a `fix:` note.

```text
emit-ts <spec dir> <out dir> [--normalize] [--without strategy,...]
```

## Pipeline

```text
ir::Program
  -> sem::infer    typed tree: every node has a type, every core call is a CoreOp on a known receiver
  -> sem::app      the plan: world, rules with write sets, cli table, help text, store schema, tests, laws
  -> ts::lower     typed tree to swc_ecma_ast
  -> ts::print     swc_ecma_codegen, one module item at a time
  -> bin/normalize the generated project's own `biome check --write` lays the text out
```

| Module | Holds |
|---|---|
| `sem/` | target-neutral: typed tree, inference, the app plan, naming, provenance. `emit-ruby` uses it too |
| `ts/build.rs` `ts/print.rs` | short constructors for swc nodes; the printer and the comment positions |
| `ts/lower.rs` | expressions and statements, with the representation choices at the top of the file |
| `ts/product.rs` `ts/tests.rs` | one generator per kind of file |
| `ts/project.rs` | the runtime shims (`include_str!` of `runtime/`), configuration, scripts |
| `out.rs` | `OutFile`, writing a project, `provenance.json` |

No TypeScript is built by gluing strings: product and test code is a swc tree. The exceptions are files with no tree to build: `package.json`, three shell scripts, and the shims, which are copied.

## Generated from the IR, and what is a shim

| Generated per program | From |
|---|---|
| `src/<module>/<decl>.ts`, one file per `type`, `fn` and `rule` | the declaration |
| `src/app/world.ts`, `outcome.ts` | every `state` of the modules that own the app's rules; every error name |
| `src/app/commands.ts` | rule signatures, by the derivation table of `pocs/todo-app/docs/cli-vocabulary.md` |
| `src/app/store.ts` | the types of the states, and each law of the form `t.states.all(w => p)` |
| `src/app/app.ts`, `src/main.ts`, `bin/*`, `package.json` | `@bin`, `@store`, `@target` |
| `test/**/<fn>.cases.test.ts`, `test/<module>.steps.ts` and `.test.ts`, `test/support/events.ts` | `@cases`, `test`, `law` |
| `test/app/help.test.ts` | the help text the compiler derived, compared with the one the table gives at load |

| Runtime shim, the same in every project (`runtime/`) | Lifted from |
|---|---|
| `src/runtime/store/`: strict JSON reader, `flock` through `bun:ffi`, lock wait, atomic replace, session | `pocs/todo-app/out/bun/src/store`, made generic over the world |
| `src/runtime/store/codec.ts`: schema-directed reader and canonical writer, with the fast reader | written for this backend; replaces the reference's hand-written `decode.ts`, `encode.ts`, `canonical-read.ts` |
| `src/runtime/cli/`: parameter shapes, parser, dispatch, exit codes, output | `out/bun/src/cli` |
| `src/runtime/values.ts`, `text.ts`, `outcome.ts` | claims, order, equality, Set and Map helpers, Text |
| `test/support/`, `test/runtime/failure-paths.test.ts` | test board, traces, base generators; failure paths of the store |

## Representation

| spec-lang | TypeScript |
|---|---|
| `Int` | `bigint`; a size or a byte is a `number` (`Type::Size` in the typed tree) |
| `Text`, `T where claim` | `string`; `Brand<base, "Name">` plus a value `Name` with `parse` (edge) and `of` (checked, inside) |
| sum with no data | its variant names; `NAME_VARIANTS` holds them |
| record | a frozen object, `makeName(...)` applies the defaults |
| `Option[T]`, `Set[T]`, `Map[K, V]` | `T \| undefined`; a frozen sorted array; a JS `Map` kept in ascending key order |
| rule | `(w: World, ...args) => Outcome<R>`: guards against `w` in written order, then the body on a draft of the states it assigns; a refusal is a value |

## Strategies

Each is named in `provenance.json` beside the span it shaped, with the fact that licensed it, and each can be switched off (`--without`, or `Strategies::without`). With all of them off the project still passes its gate and parity; that was run once on 2026-11-24.

| Strategy | Licensed by | Does |
|---|---|---|
| `rule.pure-read` (not switchable: it is the contract) | the body assigns no state | shared lock; the store is never written |
| `draft.take-or-copy` | a rule is atomic and the process commits once | the map the store session read for this one call is written in place; any other caller's is copied |
| `store.canonical-fast-path` | the canonical bytes are fixed and this program writes them | one regular expression per record reads those bytes; anything else goes to the strict reader, which alone decides `unreadable` |
| `store.law-on-read` (not switchable) | a law `t.states.all(w => p)` | a store that breaks it is `unreadable` |
| `bytes.no-array` | `t.bytes` feeds only `all` or `any` | no byte list is built |
| `match.table` | a match of a data-less sum to constants | one frozen lookup table |
| `sort.identity` | a Set iterates ascending | `set.sort(it)` is the set |
| start-up: `--bytecode`, no `.env` or `bunfig.toml` lookup, `dlopen` on the first lock | the program is fixed at build time; `@store` is its only input besides argv | as the reference's `NOTES.md` |

Guarantees of the shims, tested in `test/runtime/failure-paths.test.ts`: a failed replace leaves the old store and no tmp file; `SIGINT`, `SIGTERM` and `SIGHUP` are deferred from the first byte of the tmp file to the rename; a lock is released on every way out.

### Measured

One machine (AMD Ryzen 7 5700X3D, Linux 7.0) on 2026-11-24, other builds running on it at the time. `hyperfine -N -w 3 -r 20`, a store of 100,000 tasks (10.7 MB), mean wall time in ms. One run of the benchmark; treat as indications.

| Command | Plain translation | All strategies | Hand-written `out/bun` |
|---|---|---|---|
| `stats` | 491 ± 102 | 185 ± 12 | 137 ± 7 |
| `list` | 524 ± 52 | 233 ± 5 | 179 ± 8 |

The generated program is about 1.3 times the hand-written one on both. The target of "within 10%" is not met.

## Provenance

- Every generated file starts with `Generated by spec-lang build from <kind name (file.slg:line:col)>, ...`.
- Every guard, body statement, table row and test step has a comment above it: `slg: <file>:<line>:<col> <declaration> <role>`.
- `provenance.json`: one row per span (file, start line, end line), with declaration, `.slg` position, role, strategy and fact. Spans tile every file. Rows are sorted by file and line.
- The formatter moves lines, so `--normalize` reads the anchors back and rewrites `provenance.json` (`out::reanchor`).

No source map is emitted: swc's codegen maps to swc source files, and Biome reformats the text afterwards, so the map would be wrong after `bin/normalize`.

## What the IR had to provide, and what is missing

Used as given: resolved names, calls with arguments in parameter order, `@guard` as an ordered list, `@cases`, `@given`, `Trace` rule sets, `@target` `@bin` `@store`, spans. Missing, and built here instead: expression types. See [`IR-NEEDS.md`](IR-NEEDS.md).

## Open choices this backend settled

| Question | Chosen |
|---|---|
| Store layout | `version`, then scalar states, then collections, each in declaration order; `Map[K, R]` whose record's first field has type `K` is an array of records. Any other map is `E-OPEN` |
| Environment variable for the store | `<BIN>_STORE` |
| Help placeholder | a data-less sum shows its variants in lower case, any other type `<type name>` |
| Start world of a law's trace | reached by up to 20 random events of all the app's rules; then 40 events of the trace's rules; 200 seeded runs; the law is evaluated once, on the whole trace |
| A test that starts from another test | starts with nothing printed |
| A chained comparison `a <= b <= c` | `b` is evaluated twice |
| A claim on a value built inside the program (`no + 1` into `TaskNo`) | checked at run time by `Name.of`; nothing is proved |

## Supported subset

`type` (alias with or without claim on `Int` or `Text`, record, sum with no data), `state` (`Map`, `Set`, `List`, `Option`, or a scalar with an initial value), `fn`, `rule` with `@on(cli(...))`, `test` with `@given`, `law` over one `Trace`. Expressions: literals, interpolation, `and or not implies`, comparisons and `in`, `+ - *`, `if then else`, `match` on a data-less sum or an `Option`, `with`, records, `Set()`, `Map()`, tuples, and `size trim bytes contains starts_with list keys values get filter map all any sort join`. Statements: `let`, assignment to a state, to `state[k]` and to `state[k].field`, `state.remove(k)`, `out.line`, `fail`, `return`, `for`, `while`, `if`, `assert`, `->`.

## Limits

- One program has been built. The subset is what that program needed.
- No projection pushdown: `stats` builds every task. The representation changes asked for later (machine integers for `TaskNo`, a dense array for a map keyed by a counter, interned tags) are not done.
- `m[k]` on a missing key throws `Missing` instead of refusing with it; a guard has to cover it, and nothing checks that one does.
- Every declaration of every module is emitted, reachable from the app or not.
- A local or a `fn` named like a runtime helper (`at`, `put`, `size`, ...) is renamed with `_` or refused.
- Linux with glibc only, as the reference: `libc.so.6` and Linux `errno` values.
- The gate of the generated project needs its dependencies installed; `bun.lock` is the reference's, with the package name replaced.
- The numbers above come from one loaded machine and one run.
