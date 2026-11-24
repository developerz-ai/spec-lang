# spec-lang toolchain

> **Status `As of 2026-11-24`.** What runs: the parser, name resolution into a typed IR, a check stage, a lint stage, an interpreter for the subset the todo POC uses, a test engine for `@cases`, scenarios, properties and laws, and `spec-lang build`, which hands each app to the printer its `@target` names (the printers in `crates/emit-*` belong to other agents; this document does not describe them). What does not: a type checker for expressions, proofs, `task`, `event`, `flow`, units, handlers other than `out`.

The engine exists to **check, lint and test** a spec, never to run it as a program (`docs/decisions/006-interpreter-first-in-rust.md`).

## Run it

```bash
src/bin/check                                    # the gate; CI runs exactly this (wrap in `heavy` locally)
cd src && cargo build --release -p spec-lang-cli
target/release/spec-lang parse ../docs ../pocs/todo-app/src ../README.md
target/release/spec-lang test ../pocs/todo-app/src            # parse, check, lint, test
target/release/spec-lang test ../pocs/todo-app/src --json     # the same, one JSON object per line
target/release/spec-lang view ../pocs/todo-app/src board.done # one declaration and every check that reaches it
target/release/spec-lang build ../pocs/todo-app/src --out gen [--app TodoRust]  # check, then print each app
src/bin/lark-diff 1 5000                         # differential fuzz against the Lark grammar (needs uv)
```

`spec-lang test` options: `--seed N` (0), `--runs N` per property (100), `--traces N` per law (200), `--steps N` per trace (40), `--jobs N` worker threads (every core), `--json`, `--deny warnings`. Exit 0 when every stage is green, 1 when a check fails or a stage has errors, 2 on usage.

## Stages of `spec-lang test`

| Stage | Does | Fails on | Codes |
|---|---|---|---|
| parse | the grammar of `docs/idea/25-grammar.md`, exactly | text not in the grammar | `E-PARSE*`, `E-LOAD` |
| check | modules by path, `use` (no cycles), every name resolved, another module's names `pub` (tests and states excepted), every note known and in its place, `@guard` clauses are `cond else Error`, call and `@cases` arity, state written only by its own module's rules, `@given` setting only its own module's state, a core free name not called where a local has taken it, `pub` rules' `@uses` cover the effects inferred from their bodies (transitively), defaults and constants of their declared types | any error; the pipeline stops | `E-NAME-*`, `E-NOTE-*`, `E-TYPE-*`, `E-STATE-FOREIGN`, `E-EFFECT-*`, `E-GUARD-FORM`, `E-UNSUPPORTED` |
| lint | `pub` rule without `@why`, unused `use` and private declarations, a test with no `assert` and no `->`, an `@uses` effect the body never has, a law whose trace names rules that write none of the states it reads | only with `--deny warnings` | `W-LINT-*` |
| test | every `@cases` row, scenario, property and law | any failure | `F-*`, `E-CLAIM`, `E-RUNTIME`, `E-OPEN`, `E-GEN` |

Every code is in [`crates/diag/codes.md`](crates/diag/codes.md) with what it enforces; a unit test fails if code uses one that is not registered.

## For agents

One read of a diagnostic says where, what and what to do:

| Field | Example |
|---|---|
| `code` | `E-EFFECT-UNDECLARED`, stable |
| `span` | `todo/board.slg:47:10-47:14`, start and end |
| `decl`, `at` | `todo.board.done`, `guard[1]` / `body.stmt[2]` / `case[0]` / `given[0]` |
| `expected`, `found` | in source terms: `@uses(n.write, log.write)`, `Title`, `2` |
| `edits` | span plus replacement text, when the fix is mechanical (a misspelt name, a missing `@why`) |
| `alternatives` | each possible fix when two exist (`E-OPEN`: `/` on `Int` floors or truncates) |
| `notes` | for a law: the shrunk counterexample as a pasteable `test`, the step where it broke, the state entries that changed, the seed |

Human output is rustc-style with the source line and carets; `--json` gives the same fields, one object per line, plus one object per stage and per check. `spec-lang view <project> <decl>` prints a declaration, what it names, what names it, and every case, test and law that reaches it with its result. `view --as pseudo|diagram` forwards to `crates/view`.

## Architecture

```text
.slg ──► syntax ──► ir ──► check (errors, warnings)
                     │
                     ├─► engine ──► testing (cases, scenarios, properties, laws) ──► report
                     ├─► view   (projections)
                     └─► emit-* (printers per backend)
          diag: one diagnostic shape for every stage        cli: the `spec-lang` binary
```

| Crate | Concern | Modules |
|---|---|---|
| `syntax` | text to AST; no meaning | `token` `lexer` `unicode` (Python `re` tables) `parser/{decl,stmt,expr}` `ast` `span` `diag` `fence` |
| `diag` | the diagnostic shape; human and JSON renderings; the code registry | `lib` `render` `json` `codes.md` |
| `ir` | path modules, `use`, name resolution, lowering to the resolved program; **the public API emitters and views read** | `load` `lower/{declare,decls,ty,expr,stmt,pattern,text,notes,scope}` `program` `expr` `ty` `ids` `visit` `vocab` |
| `check` | effects inferred against `@uses`; defaults; lint; per-declaration references | `effects` `defaults` `lint` `refs` |
| `engine` | values, worlds, evaluation, atomic rules, claims, the core library | `value` `world` `interp` `eval/{call,ops,pattern,stmt}` `conform` `stdlib/{seq,text}` `display` |
| `testing` | generation, shrinking, scenarios, properties, traces, laws, missing-law offers, the report | `generate` `shrink` `rng` `parallel` `scenario` `property` `trace` `law` `missing` `runner` `report` `config` |
| `cli` | `parse`, `test`, `view`, `build` (depends on every `emit-*` crate) | `main` `args` `sources` `output` `parse_cmd` `test_cmd` `view_cmd` `build_cmd` |
| `emit-rust` `emit-ts` `emit-ruby` `emit-cpp` | reserved for backend printers over `ir` (`docs/idea/11-backends-and-parity.md`) | owned by other agents |
| `view` | projections over `ir` | owned by another agent |

`ir` entry point: `spec_lang_ir::load_project(&Path) -> Result<Program, LoadFailure>`. `Program` holds typed tables (`types`, `states`, `consts`, `fns`, `rules`, `tests`, `laws`, `apps`, `lambdas`) addressed by ids; rules carry `guards: Vec<Guard { cond, error }>` in written order, `uses`, `triggers`; apps carry `rules`, `target`, `bin`, `store`. Expressions name what they mean (`Local`, `State`, `Const`, `Variant`, `Call { callee: Fn | Rule | Record | Variant | Effect, args in parameter order }`); member access on values (`Prop`, `Method`) stays dynamic because expressions are not typed yet.

## What the engine runs

The subset `pocs/todo-app/src` uses, and the neighbours that cost nothing more: `type` (alias, range, record with defaults, sum, `T where claim`, rule set), `const`, `state`, `fn`, `rule`, `test`, `law`, `app`; `use` of a module, of a declaration (a sum type brings its variants), of the `cli` vocabulary; `@why @guard @uses @on(cli(..)) @cases @given @target @bin @store`; `let var = for while if match return assert`, `x else v`, `x else fail E`, `fail E`, lambdas with `it`, text interpolation and escapes, `== != < <= > >= in and or not implies`, `+ - * mod ^ band bor bxor shl shr`, ranges, tuples, `List Set Map Option`, `.is(pattern)`, `.with(...)`, and the library names `size bytes first last keys values list get filter map flat_map fold sum min max all any find position contains distinct sort reversed take drop zip group_by join trim lower starts_with split replace abs clamp push insert remove`. A trace gives `t.steps` (`event`, `reply`, `before`, `after`) and `t.states`; `s.event.no` reads an argument by name.

Anything else is `E-UNSUPPORTED` at `file:line`, never a silent answer: units and decimals, prefixed text (`re".."`), `/` on `Int` (`E-OPEN`), `task event flow note kind handler`, `machine` and `foreign` bodies, `@pre @post @access @edge @retry` and other notes, handlers such as `clock`, `now()`, `t.eventually`, `at` of a step, notes on parameters, generics, a `.is` pattern that binds names.

`Int` is `i128`; leaving it stops evaluation with `E-RUNTIME` rather than wrapping. `Text.size` counts code points; `trim` removes exactly Unicode `White_Space` (Rust's `char::is_whitespace`, 25 code points, checked by a unit test). `Map` and `Set` iterate in ascending order; values order as `docs/idea/28-standard-library.md` fixes. A rule called from outside is atomic: on a failure the world and its unsent `out.line` lines are restored.

## The test engine

| Form | Run as | Evidence printed |
|---|---|---|
| `@cases` row | one call | `example` |
| `test` with no parameters | `@given` world, then the steps; a `@given` test's end world is reused, and a failing parent blocks its children | `example` |
| `test` with parameters | 100 runs, parameters generated from their types and claims, seeded per test; failures shrunk | `sampled(100)` |
| `law` | 200 traces. Even ones start from a world reached by up to 10 random calls of the rules of the traced rules' modules; odd ones from a **random lawful world**: every state drawn from its type, claims included, kept only if every law of the program holds on it (up to 200 tries, else the trace starts reached). Then 40 random calls of the traced rules; arguments generated from parameter types, half of them picked from keys already in the world; other law parameters generated after the trace from the keys it saw | `sampled(200 traces x 40 steps, n from lawful worlds)` |

Nothing reports `proven`. Seeds derive from `--seed` and the check's name (FNV-1a, SplitMix64), so adding a check never changes another's inputs, and every run is reproducible. A law counterexample is shrunk by dropping calls, then a smaller start world, then smaller arguments, and printed as a `test` with the start-building calls above `// the trace starts here`, or with the start world as `@given`. The message says which start found it (`from a reached world` or `from a random lawful world`). From a lawful world, no rule may reach the start, so the diagnostic offers both answers (`E-OPEN` style): a state law that excludes the world, with the edit that adds it, or a wrong rule. A law is offered only when it is false on the start world and held on every world of 40 reached traces; the catalogue has one shape so far, "a map's records sit under their own key field" (`crates/testing/src/missing.rs`).

**One world per worker thread** (decision 006). The traces of a law and the runs of a property are spread over `--jobs` threads, each with its own interpreter and world. Each run is a pure function of the base seed, the check's name and the run's number; results are merged by number, never by thread, so the report is byte-identical for any `--jobs` (`crates/cli/tests/jobs.rs` compares 1 and 16 workers on the todo program and on a failing law). Runs numbered above a known failure are skipped; the failing run is then replayed on the calling thread and shrunk there. Checks themselves still run one after another.

**Measured** (method: `hyperfine -N -r 10` of `target/release/spec-lang test ../pocs/todo-app/src`, release build, 16 hardware threads; caveat: one machine, one program, no other load controlled): 35 checks (18 `@cases` rows, 9 scenarios, 1 property, 7 laws), all green.

| Build | Wall time, mean ± σ | User time |
|---|---|---|
| one thread | 1.121 s ± 0.033 s | 1.120 s |
| `--jobs` = 16 (default here) | 180.2 ms ± 15.8 ms | 1.835 s |

The extra user time is the cost of a fresh interpreter per run and of threads; it was not profiled. A world is a vector of shared roots, so copying one per step costs a few pointer copies; that was left as is.

## Proof that the tests can fail

`crates/cli/tests/mutations.rs` copies `pocs/todo-app/src` to a temporary directory, applies one edit, runs the real binary with default counts, and records which checks failed. Last run:

| Mutant | Caught by |
|---|---|
| `next_no = no + 1` → `next_no = no` | law `numbers_below_next`, tests `numbers_start_at_one`, `numbers_are_never_reused` |
| `done`'s two guards swapped | property `unknown_numbers_are_refused` |
| `t.status == Open` → `true` in `list` | test `list_ranks_urgent_first` |
| `tasks.remove(t.no)` dropped from `clear` | law `clear_keeps_every_open_task`, test `clear_removes_only_done` |
| `reopen` guard checks `Open` | test `done_and_reopen_move_once` |
| `done` sets `Open` | test `done_and_reopen_move_once` |
| `rename` keeps the old title | test `rename_keeps_the_rest` |
| `remove` without its guard | property `unknown_numbers_are_refused` |
| `stats` counts everything as done | test `clear_removes_only_done` |
| `has_tag`: `none => false` | test `list_ranks_urgent_first` |
| `rank` by number first | test `list_ranks_urgent_first` |
| `!high` badge misspelt | case `line`, tests `list_ranks_urgent_first`, `list_all_and_by_tag` |
| tags accept `_` instead of `-` | case `is_tag` |
| titles up to 2000 code points | **survives**: no case or test has a title of 201 to 2000 code points; generated titles are at most 12 |
| `add` stores `Normal` whatever the priority | **survives**: every scenario that reads a priority sets it through `@given`, and no law relates `add`'s arguments to the stored task |

The test fails if a mutant's fate changes, so the two survivors stay listed until a test or law catches them.

## Agreement with the reference parser

- Every `slg` fence in `docs/` and `README.md` and every `.slg` of the todo POC parses (120 fences, 9 files when last run; `crates/cli/tests/parse_docs.rs`), as with `pocs/todo-app/scripts/slg/check.py`.
- `src/bin/lark-diff` mutates those texts and compares which mutants each parser rejects. After the last parser change: 8 seeds × 5,000 mutants plus 1 × 3,000, identical rejected sets every time. Earlier runs found three disagreements, each now reproduced (see `crates/syntax/src/parser/mod.rs`): `pub` is a keyword right after any path; statement keywords are keywords right after a value; lexing restarts after a split keyword (`else100%`). Caveat: random token edits, not a proof of equal languages.
- The lexer uses Python `re` character classes for `\w`, `\d` and `\s`, generated from CPython 3.15.2 (Unicode 17.0.0) into `crates/syntax/src/unicode_tables.rs`. A reference checker on another Python can differ on letters new to Unicode.

## Findings in the todo program

- The program lacked a state law. Traces from random lawful worlds made `clear_keeps_every_open_task` false after `board.done(47); board.clear()`, from a world where `tasks[47]` holds `Task(1, ...)`: `clear` removes by `t.no`, not by key. No rule reaches that world, but no law excluded it. The engine offered `t.states.all(w => w.tasks.keys.all(k => w.tasks[k].no == k))`, and the program now has it as `stored_under_its_number`.
- Earlier lint findings (`pub` rules without `@why`, a law over rules that cannot change what it reads, `@given` setting another module's state) were fixed in the program.

## Settled in the card

Questions this engine raised that `docs/idea/01-language-card.md` now answers, implemented as written there: a step's `reply` is `Ok(v)` or `Err(e)` (`fail(...)` is no pattern; the diagnostic offers the `Err(...)` edit); `event` is the call (`s.event.no`, `s.event.is(board.clear())`, unwritten arguments match anything); a rule's name is its event type, so `type Change = a | b;` names a set of events; a failure while evaluating a guard's condition gives that guard's error; core free names are not reserved, but a body whose local took one cannot call it; traces also start from any world the types and laws allow; `pub` is enforced across modules (`E-NAME-PRIVATE`); `@given` sets only its own module's state, and another module's world is named as that module's test.

## Open choices the engine made

Each is a place where the program or the docs allow two behaviours. The engine picked one; no doc decides it yet.

1. **What needs `pub`.** `30-type-rules.md:41` says another module's name must be `pub`. Chosen: types, constants, `fn`s and rules need it; tests (named by `@given`) and states (read by tests and laws, written only by their own rules) do not, because the grammar gives no way to read a world otherwise and the program reads `board.tasks` from its scenarios (`scenarios.slg:51,59`).
2. **`sort` on a `Set`.** `render.slg:20` writes `tags.sort(it)`; `28-standard-library.md:179` lists `Set` methods without `sort`. Chosen: `Set.sort(key)` gives a `List`.
3. **`out` without `use cli`.** `scenarios.slg:11` reads `out.text` and its module has no `use cli`. Chosen: `out.text` is a test handler available to every test; `out.line` needs `use cli` in the rule's module.
4. **`out.text` in a test started from another test.** Chosen: it starts empty in every test.
5. **Which laws a lawful world must meet.** The card says "the laws". Chosen: every law of the program, each on the single world (step laws are vacuous there); other law parameters are generated once per check.
6. **`x else v`: failure or `none`?** `28-standard-library.md:23` decides by type. Without expression types the engine decides by form: when `x` is an index (`m[k]`) only a failure is handled; otherwise a failure, and a `none` (with `Some(v)` unwrapped).
7. **Claims are monitored, not only obligations.** The engine checks claims, fields of records and variants included, on every argument and result of a `fn` and rule, every record or variant built, every state write and every `@given` value; a broken claim is `E-CLAIM`, never a refusal.
8. **Evidence of one example.** The card's ladder has no level for a single run; cases and scenarios print `example`.
9. **Defaults in generated calls.** Traces and properties generate every parameter, defaults included.
10. **Printing data in text.** `28-standard-library.md:136` says `"{x}"` prints any data without a format. Chosen: Text raw, anything else as spec-lang source (`Some("a")`, `Set(1, 2)`).
11. **`Int`.** Chosen: `i128`, stopping with `E-RUNTIME` outside it; the language says unbounded.
12. **Order of `Option` and `Result`.** Declared order: `Some(_) < none`, `Ok(_) < Err(_)`.
13. **`it` binds to the nearest library call that takes a function** (`filter`, `map`, `sort`, `all`, ...), found by name because expressions are not typed: in `sort(rank(it))` the lambda is `sort`'s argument, not `rank`'s.
14. **`spec-lang build` runs parse and check, not the tests.** `spec-lang test` is the gate before a build; a build of a red program is allowed.

## Limits

- No type checker for expressions: a mistyped expression is found when it runs, as `E-TYPE-MISMATCH` or `E-RUNTIME`, and only on the paths a test reaches.
- Sampling, not proof: 200 traces of 40 steps found every mutant above that a law can see, and nothing says they would find a deeper one. The two surviving mutants show what the suite does not cover.
- Lawful worlds come from rejection sampling: a law that few random worlds meet leaves most odd traces starting reached (the count is in the evidence). The missing-law offer knows one shape of law.
- Parallelism is inside a check (its traces or runs), not across checks; a program with many small checks gains little.
- Property reports do not count runs that returned early (`unknown_numbers_are_refused` returns for `no <= 4`).
- Name resolution of `NAME` characters follows Python's tables only as of Unicode 17.0.0.
- The effect inference covers state reads and writes and `out.line`; other effects are not modelled.
- `spec-lang view <decl>` runs every check to show results; there is no cache of results yet.
- Numbers about speed come from one machine and one program.
