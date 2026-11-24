# emit-cpp

The C++ backend of `spec-lang build`. `emit(program, app)` turns one app of a resolved program into a C++23/CMake project; `emit_with(program, app, &Options)` does the same with named strategies switched off. Output for the todo POC: `pocs/todo-app/gen/cpp/`.

## Pipeline

```text
ir::Program ──front::build──> typed front (shared with emit-rust: types, facts, cli table)
            ──lower::*─────> C++ target tree (ast.rs: decls, statements, expressions)
            ──ast printer──> plain C++ with chunk markers
            ──clang-format─> final layout; markers removed, their lines become provenance spans
            + runtime/ shims (verbatim) + CMakeLists.txt, bin/, provenance.json
```

| File | Does |
|---|---|
| `src/ast.rs` | the typed C++ tree and its printer; expressions carry precedence, so parentheses appear only where C++ needs them |
| `src/format.rs` | one `clang-format` run per file (style `STYLE`, also written as the project's `.clang-format`); markers `// @@slg N` start a chunk span, anchors `// slg: file:line role` start a nested span and stay in the output |
| `src/lower/model.rs` | sums as `enum class` with name tables; refined types as classes whose only way in is `make` (the claim, checked once); records as aggregates; fns `constexpr` where their body allows |
| `src/lower/board.rs` | the world, the errors guards name, one event per rule, one source per rule (guards in written order, then the body, writing the world in place), dispatch |
| `src/lower/cli.rs` | the value types, the derived parameter table, the help text (folded at build time), binding parsed values to events |
| `src/lower/store.rs` | the store codec, generated from the states' types: schema-directed reader, canonical writer, invariants checked on read |
| `src/lower/tests.rs` | every `@cases` row, every `test` by name (with parameters: a seeded property), every `law` over seeded traces |
| `src/lower/project.rs` | module headers in dependency order, the app type the runtime driver reads, `main`, CMake, scripts, `provenance.json` |
| `runtime/` | the shims, emitted verbatim |

## Generated against runtime shim

| Generated from the IR | Runtime shim (`runtime/`, lifted from `pocs/todo-app/out/cpp`) |
|---|---|
| model types and their claims, fns, rules, world, events, dispatch | `rt/unicode`, `rt/text` (Text.size, trim, building text), `rt/core` (length, get, pointers to elements) |
| the cli value types, parameter table, help text, binding | `rt/cli.hpp` (the parsing order of contract section 2, over any table), `rt/exit_code.hpp` |
| the store codec: one reader and writer per type in the states | `rt/json_lexer`, `rt/json_reader.hpp`, `rt/json_writer.hpp`, `rt/flat_map.hpp` |
| `app::App`, `main` (store path from `@store`, variable `<BIN>_STORE`) | `rt/driver.hpp` (lock, read, rule, atomic replace, error table), `rt/main.hpp`, `rt/file`, `rt/lock`, `rt/fd.hpp` |
| tests, laws, generators, literal helpers (`tests/support.hpp`) | `tests/rt/harness`, `tests/rt/random.hpp`, `tests/rt/trace.hpp` |

The store format mapping is fixed per backend, not per program: `version` then one key per state, states sorted by name; a record is an object with its fields in declaration order; a sum without data is its variant's name; a `Set` is an ascending array, a repeat rejected; a `Map` whose values sit under their own key field is the array of its values in key order; an `Int` is an integer from 0 to 2^53, a larger one is `full`.

## Strategies

Each is licensed by a fact of the shared front, named on its span in `provenance.json`, and can be switched off (`Options::off`, names from `spec_lang_emit_rust::STRATEGIES`); off, the plain translation is emitted instead.

| Strategy | Fact | Plain translation |
|---|---|---|
| `entry.lookup` | first guard `k in m`, the rule reaches `m` only as `m[k]` | `contains`, then `m.at(k)` at each use |
| `entry.remove` | first guard `k in m`, the only other use is `m.remove(k)` | `contains`, then `erase` |
| `filter-remove.retain` | `let g = m.values.filter(p); for x in g { m.remove(x.f) }`, `m` keyed by `f`, `g` used after only for its size | pointers to the matches, then one `erase` each |
| `store.read.projection` | a read-only rule reads the keyed map's values only through some scalar fields, and so do the invariants | the full decode |
| `sort.unstable` | the sort key ends in the field that keys the map | `std::ranges::stable_sort` |
| `link.static` | `@why("... one static binary")` | dynamic link |

Not switchable, because the contract requires it of every translation: a read-only rule takes the shared lock and never writes the store; a byte-identical store is not rewritten.

## What the IR had to provide

Resolved names and declaration tables (`spec-lang-ir`); everything else came from the shared front in `emit-rust`: expression types, refinement changes made explicit, the facts above, the derived cli table. Computed here because the front does not carry it: placement of fns (header `constexpr`, source, or inside the one rule that uses it), the module dependency order, the generators' value pools.

## Open choices

| Question | Chosen | Why |
|---|---|---|
| `Int` | `std::int64_t`; a `TaskNo` argument past 64 bits saturates | every stored number is at most 2^53, so a saturated one is in no world |
| Text | `std::string`, valid UTF-8 by invariant: the cli parser and the store reader check it where bytes come in | one check per edge, not one per value |
| Where 2^53 is enforced | the writer, as `full` | the rules stay exact; the store refuses a world it cannot carry |
| Help text | a string literal computed by the compiler | the out/cpp projection folds it with `consteval`; same bytes, less code |
| Law traces | 240 traces of 40 steps; one in three starts from the end world of a test used as a `@given` | the card allows any world the rules can reach |
| Property runs | 200 per test | none stated |

## Limits

- One program has been through this backend. Forms outside its subset are refused with `E-UNSUPPORTED`; a second program will find more.
- `clang-format` runs during `emit`, so the bytes depend on its version; the generated `bin/check` runs the same check.
- Law traces and properties are sampled, not proven.
- Each strategy is licensed by a fact with a written reason, not by a proof; the parity suite and the generated tests are the evidence that each is equivalent to its plain translation.
- Measurements are one machine, one synthetic store of 100 000 tasks, warm cache.
