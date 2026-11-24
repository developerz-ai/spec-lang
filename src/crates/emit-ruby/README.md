# emit-ruby: the Ruby backend

> **State `As of 2026-11-24`.** Runs for one program: the todo POC (`pocs/todo-app/src`). `emit` turns its `@target(ruby)` app into a Ruby project whose own gate passes and which reproduces the POC's 68 parity transcripts. Any form outside the subset is an `E-UNSUPPORTED` diagnostic, never a guess.

`pub fn emit(program: &ir::Program, app: &ir::AppDecl) -> Result<Vec<OutFile>, EmitError>`. Same input, same bytes.

```text
emit-ruby <spec dir> <out dir> [--without strategy,...]
```

## Pipeline

```text
ir::Program
  -> emit_ts::sem   typed tree and the plan of the app, shared with emit-ts
  -> rb::lower      typed tree to rb::ast, a small typed Ruby tree
  -> rb::print      two spaces, double quotes, a long list one item per line
```

| Module | Holds |
|---|---|
| `rb/ast.rs` `rb/print.rs` | the Ruby tree (module, class, def, `Data.define`, blocks, lambdas, `case`) and its printer |
| `rb/file.rs` | one file: namespace, `require_relative` lines derived from what it references, provenance |
| `rb/lower.rs` | expressions and statements, with the representation choices at the top of the file |
| `rb/product.rs` `rb/tests.rs` | one generator per kind of file |
| `rb/project.rs` | runtime shims (`include_str!` of `runtime/`), `Gemfile`, `exe/<bin>`, scripts |

Every Ruby file the tests emit passes `ruby -wc` and parses with Prism (`tests/todo.rs`).

## Generated from the IR, and what is a shim

| Generated per program | From |
|---|---|
| `lib/<module>/<decl>.rb`, one file per `type`, `fn` and `rule`, in `Module::Path` | the declaration |
| `lib/app/world.rb`, `refusals.rb` | every `state` of the modules that own the app's rules; every error name |
| `lib/app/commands.rb` | rule signatures, by the derivation table of the `cli` vocabulary |
| `lib/app/store.rb` | the types of the states, and each law of the form `t.states.all(w => p)` |
| `lib/app/cli.rb`, `exe/<bin>`, `bin/*`, `Gemfile` | `@bin`, `@store`, `@target` |
| `test/**/<fn>_cases_test.rb`, `test/<module>_steps.rb` and `_test.rb`, `test/support/events.rb` | `@cases`, `test`, `law` |

| Runtime shim, the same in every project (`runtime/`) | Lifted from |
|---|---|
| `lib/runtime/store/`: strict JSON reader, `File#flock` with a bounded wait, atomic replace, file store | `pocs/todo-app/out/ruby/lib/todo/store`, made generic over the world |
| `lib/runtime/store/codec.rb`: schema-directed reader and canonical writer, with the fast reader | written for this backend; replaces the reference's `decoder.rb` and `encoder.rb` |
| `lib/runtime/cli.rb`, `cli/`: table rows, argument types, parser, dispatch, exit codes | `out/ruby/lib/todo/cli` |
| `lib/runtime/values.rb`, `text.rb` | claims, refusals, outcomes, Map order, stable sort, Text |
| `test/support/testing.rb`, `test/runtime/failure_paths_test.rb` | test board, seeded generators, traces; failure paths of the store |

## Representation

| spec-lang | Ruby |
|---|---|
| `Int`, `Text` | `Integer`; a frozen UTF-8 `String` |
| `T where claim` | the base value; a module `Name` with `valid?`, used at every edge and by every constructor that holds one |
| sum with no data | the symbol of the variant; `Name::ALL` |
| record | `Data.define` whose `initialize` applies the defaults and checks every claim, so `with` checks them again |
| `Option[T]`, `Set[T]`, `Map[K, V]` | `T` or `nil`; a frozen `Set`, sorted where order shows; a `Hash` kept in ascending key order |
| rule | `Module.rule(world, **params) -> Runtime::Outcome`; a refusal is an exception raised before any new world exists |

## Strategies

Named in `provenance.json` with the licensing fact; each switchable with `--without`. With all of them off the project still passes its gate and parity; that was run once on 2026-11-24.

| Strategy | Licensed by | Does |
|---|---|---|
| `rule.pure-read` (not switchable) | the body assigns no state | shared lock; the store is never written; the atomic writer is never loaded |
| `draft.take-or-copy` | a rule is atomic and the process commits once | the Hash the store read for this one call is written in place; any other caller's is `dup`ed |
| `store.canonical-fast-path` | the canonical bytes are fixed and this program writes them | a `StringScanner` reads those bytes straight into values; anything else goes to the strict reader |
| `store.law-on-read` (not switchable) | a law `t.states.all(w => p)` | a store that breaks it is `unreadable` |
| `match.table`, `sort.identity` | as in `emit-ts` | a frozen Hash; `set.sort` |
| start-up: `--disable-gems`, lazy requires | no gem at run time; parsing is derived from a rule's parameters, not its body | help and usage errors load the parameter table only; a command loads its own rule file, and the store only when it runs |

Guarantees of the shims, tested in `test/runtime/failure_paths_test.rb`: a failed replace leaves the old store and no tmp file (`ensure`); an interrupt is deferred from the first byte of the tmp file to the rename (`Thread.handle_interrupt`); a lock is released on every way out; the write buffer is sized once from the bytes read.

### Measured

Same machine, day and store as in `emit-ts/README.md`; `hyperfine -N -w 1 -r 6`, mean wall time in ms, through the version manager's `ruby` on `PATH`. One run; the machine was busy.

| Command | Plain translation | All strategies | Hand-written `out/ruby` |
|---|---|---|---|
| `stats` | 2093 ± 380 | 1467 ± 44 | 1013 ± 69 |
| `list` | 2254 ± 22 | 1825 ± 54 | 1607 ± 99 |

`stats` is 1.45 times the hand-written program, which reads counts without building tasks; this backend has no such projection. `list` is 1.14 times.

## Open choices

Those of `emit-ts/README.md`, and:

| Question | Chosen |
|---|---|
| Namespaces | a spec module `a.b` is `A::B`; `App` and `Runtime` are the backend's, so a spec module under `app` or `runtime` is refused |
| Arguments | a `fn` takes parameters without a default by position and the others by keyword; a rule and a record take all by keyword |
| Linting | none. The reference ran rubocop; the printer does not keep lines under a limit, so the gate is `ruby -wc`, minitest under `-w`, build |

## Limits

- One program has been built. The subset is the one in `emit-ts/README.md`.
- No projection pushdown, no interned tags, no symbols-as-integers; the representation is the plain one above.
- `map[k]` on a missing key raises `KeyError` instead of refusing with `Missing`.
- No `Gemfile.lock` is emitted. `bundle check` resolves against the gems installed on the machine; minitest 6.0.0 ships with Ruby 4.0, and its dependency `prism` is installed by `bundle install` if missing.
- Steps of tests are instance methods of `<Module>Steps`; two modules with a test of the same name included in one class would collide.
- This crate depends on `emit-ts` for the shared plan; that code should move (see `emit-ts/IR-NEEDS.md`).
- The numbers above come from one loaded machine and one run.
