# emit-rust

The Rust backend of `spec-lang build`. `emit(program, app)` turns one `app` of a resolved program (`spec-lang-ir`) into a complete Cargo project: relative paths and bytes, sorted, the same bytes for the same program and options. `emit_with(program, app, &Options)` switches strategies off by name. `write(dir, files)` puts the files on disk and leaves byte-identical files alone.

The todo POC is the proof: `cargo test -p spec-lang-emit-rust` emits `pocs/todo-app/gen/rust`; `--include-ignored` also runs that crate's own gate and the gate of the plain translation (every strategy off).

## Pipeline

```text
ir::Program + AppDecl
  -> front::build      typed tree (front/typer.rs), facts (front/facts.rs), cli table (front/cli.rs)
  -> rust::layout      where each declaration lands: one file per type, per rule, small groups
  -> rust::lower       typed tree -> Rust token trees, with value categories
  -> rust::items, table, codec, tests, project
  -> rust::print       parse as syn, print with prettyplease, anchors -> `// slg:` comments
  + runtime/           shims, emitted verbatim
  + provenance.json
```

**Target trees, never string glue.** Every item is built with `quote`, parsed back as a `syn` item before printing (a printer bug fails the emit, not rustc), and printed with `prettyplease` (feature `verbatim`, so `format!` and `assert_eq!` arguments are laid out too). The tests parse every emitted `.rs` file again with `syn::parse_file`.

**Value categories.** Each lowered expression is owned, a reference of some depth, a place, an `Option<&T>`, or an iterator (`rust/lower.rs`, `Cat`). That decides every `&`, `*`, move and clone: a rule parameter used once moves into the record it builds; one used twice is cloned; a `values()` pipeline stays an iterator until a `let` collects it.

**`front/` is shared** with `emit-cpp`, which depends on this crate for it. It is SIR-F work the IR crate does not do yet; [IR-NEEDS.md](IR-NEEDS.md) lists each piece and where it belongs.

## Generated or runtime shim

| Generated from the IR | Runtime shim (`runtime/`, verbatim) |
|---|---|
| `src/model/*`: sums with `ALL`, `name`, `from_name`; refined types as newtypes whose only constructor checks the claim (`new`, and `admits` for text checked without building it); records | `src/lang.rs`: `Text.size`, `Text.trim` with the contract's 25 white-space code points, `Int` from a length, `Entries` (a key-ordered map read) |
| fns, `const fn` where the body allows (`mark`, `badge`, `urgency`, `rank`); `&'static str` where every result is a literal | `src/out.rs`: the `out.line` buffer, written after commit |
| `src/board/*`: `World`, `Refusal`, one file per rule, `Command` and `apply`; for a projected rule its view types | `src/cli/{params,parse,run,usage,exit}.rs`: the parser of contract section 2 over any table, the error table, exit codes |
| `src/cli/table.rs`: value types, the static command table, binders, the help text as a `const` | `src/store/{cursor,file,lock,atomic,path,writer,error,unreadable}.rs`: strict JSON cursor, `flock` with patience, atomic replace, a store generic over `Decode`/`Codec` |
| `src/store/codec.rs`: decode and encode from the states' types, a fused reader per projected rule, state invariants checked on read | `tests/support/rng.rs`: SplitMix64 and the text pool; `tests/store_failures.rs`: the store's failure paths (package name filled in) |
| `src/app.rs`, `src/main.rs`, `src/lib.rs`, `Cargo.toml`, `Cargo.lock`, `.cargo/config.toml`, `clippy.toml`, `bin/build`, `bin/check` | |
| `tests/*_cases.rs` (every `@cases` row), `tests/<module>.rs` (every `test` by name; one with parameters is a seeded property, 300 runs), `tests/laws.rs` (every law after every step of 240 seeded traces of 40 calls), `tests/support/mod.rs` (named start worlds, one generator per type) | |

Every file starts with a header naming its declarations and their `slg` file:line:col, or the shim. `provenance.json` maps every span of emitted lines (file, start, end) to its declaration, `slg` location, role (`type`, `fn (const)`, `rule`, `guard[1]`, `cli param table row`, `case[2]`, `law body`, `runtime shim store.lock`, ...) and, where one applied, the strategy and the fact that licensed it. A test checks that every line of every generated file is in some span. Guards and strategy sites carry a readable anchor: `// slg: src/todo/board.slg:46 guard[1]`.

## Strategies

Named, each with its precondition from `front/facts.rs`, recorded with its licence in `provenance.json`, and each switchable off (`Options`, `STRATEGIES`); off, the plain translation is emitted. Both the default and the plain translation are tested against parity.

| Strategy | Licence | Effect | Default |
|---|---|---|---|
| `entry.lookup` | guard 0 is `k in m` and every later use of `m` is `m[k]` | one `get_mut(&k).ok_or(E)?` serves both guards and the write | on |
| `entry.remove` | guard 0 is `k in m` and the only use is `m.remove(k)` | one `remove(&k).ok_or(E)?` | on |
| `filter-remove.retain` | `m` is keyed by a field of its values; the filtered list is used after the loop only for its size | `retain` and a length difference; the list is never built | on |
| `store.read.projection` | a read-only rule reads a keyed map only by size, by key and through scalar fields of its values, and every state invariant is decidable from the same fields | `stats` gets its own reader: the whole store is checked, only `no` and `status` are kept, in a flat `Entries`; no `World` is built | on |
| `sort.unstable` | the sort key ends in the field that keys the map (`rank` ends in `t.no`) | `sort_unstable_by_key` | on |
| `sink.inline-format` | `out.line(f(t))` with `f` pure and one interpolation | `format_args!` into the buffer; no `String` per line | on |
| `link.static` | the app's `@why("... one static binary")` | `+crt-static`, fat LTO, `panic = "abort"`, stripped | on |
| `text.inline` | a refined text whose claim admits only ASCII and `size <= n` (`Tag`: 32) | held in `[u8; n]` and a length, no heap | **off**: measured, not a clear win (below) |

Not switchable, because the contract requires them of every translation: a read-only rule (empty write set, `@uses` agrees) takes the shared lock and never writes the store (`rule.read-only` in provenance); `cli.derive`; `store.json.schema`. Switching `rule.read-only` off cost two parity cases (a read created and rewrote the store), which is how it was moved out of the list. Claim discharge (`no + 1` with `no >= 1` needs no check: `TaskNo::from_proven`) is required for the program to be emitted at all.

Declined, with the reason: a slab (`Vec<Option<Task>>` indexed by number) for a map whose keys come from the owned counter. A store read from outside may hold any key below a `next_no` of up to 2^53, so a slab sized by `next_no` is unbounded; `NonZeroU64` for `TaskNo`, because no `Option<TaskNo>` is stored anywhere, so its niche saves nothing.

Peepholes in the printer: `xs.size == 0` is `is_empty()`, `lo <= x <= hi` is `(lo..=hi).contains(&x)`, two sizes compare as raw lengths, `p = p - 1` is `p -= 1`, a comparison under `not` is flipped, sorting a `Set` by its own element and `Set.list` are its iterator, `join("")` collects into a `String`.

## Result on the todo program

Measured 2026-11-24 on one Linux x86-64 machine with Rust 1.100. The suite is the coordinator's current one: 68 cases after the spec change that forbade line breaks in titles and a leading `-` in tags.

| Check | Command | Result |
|---|---|---|
| Emit | `cargo test -p spec-lang-emit-rust` | 17 tests pass; `--include-ignored` adds the two generated gates |
| Generated gate | `gen/rust/bin/check`: clippy `-D warnings` on all targets, `cargo test`, release build | passes: 9 scenario, 1 world, 4 `@cases`, 7 law, 4 store-failure tests |
| Parity | `bun scripts/parity/run.ts` from a scratch root whose `backends.json` names the generated binaries | 68/68, default and plain |
| Race | the same, `--race 40` | numbers 1..40, none lost, both |

When the spec changed, the generated backend followed by re-emitting, without edits; the hand projections, written against the old spec, did not.

**Speed and memory**, on a 100,000-task store (10.7 MB), cwd at the store. Times are `hyperfine -N -w 3 -r 20`; the machine was shared with other agents (load average up to 12 during some runs), so σ is often several ms and single runs differ by more than the gaps below. Allocations come from `valgrind --tool=memcheck` ("total heap usage") on dynamic builds of the same source, since valgrind cannot intercept the static binary's allocator; peak RSS comes from `/usr/bin/time -f %M` on the shipped static binaries.

| `stats` | time | allocations | bytes allocated | peak RSS |
|---|---|---|---|---|
| hand `out/rust` | 20.6 to 23.3 ms | 38 | 12.8 MB | 12.4 MB |
| generated, plain | 50.7 to 59.9 ms | 283,354 | 39.5 MB | 46.0 MB |
| generated, default | 23.6 to 25.9 ms | 37 | 14.9 MB | 13.3 MB |

| `list` | time |
|---|---|
| hand | 56.0 to 59.1 ms |
| generated, before `sink.inline-format` | 85.2 ms |
| generated, default | 64.4 to 69.2 ms |

The coordinator's target was the generated `stats` within 10% of the hand-written one. It is not met: in the last paired run `stats` was 23.9 against 20.6 ms (+16%) and `list` 64.7 against 57.2 ms (+13%). The hand projection counts statuses while parsing, keeps no array, and fuses `is_title` into one pass with an 800-byte early exit. The generated reader keeps `(no, status)` per task, because the invariant `stored_under_its_number` reads `no`.

| `add x` | allocations | bytes allocated | peak RSS |
|---|---|---|---|
| hand | 350,060 | 75.9 MB | 55.5 MB |
| generated, default | 383,382 | 66.5 MB | 55.6 MB |
| generated, `text.inline` on | 283,381 | 78.4 MB | 64.0 MB |

Non-blank lines of `.rs` files (`grep -cv '^\s*$'`):

| | Hand (`out/rust`) | Generated (`gen/rust`) |
|---|---|---|
| `src/` | 2073 | 2112, of which 1035 runtime shim |
| `tests/` | 1555 | 734, of which 143 runtime shim |

The generated tests are the spec's own (`@cases`, `test`, `law`) plus the store's failure paths; the hand projection also unit-tested its codec and parser, which are runtime shims here and covered end to end by parity.

## Failure paths

The runtime's guarantees, tested in every generated project (`tests/store_failures.rs`, span `store.failure-hygiene`):
- A write the directory refuses leaves the old store byte for byte and no temporary file, and the lock is free afterwards.
- A refusal writes nothing.
- An unreadable store is reported and never replaced.
- A writer that holds the lock makes the next one `Contended`, then lets it in once it returns.

Locks and files are released by `Drop` on every return. The temporary file is removed on every error path of `atomic::replace`.

## Open choices

| Choice | Taken | Why |
|---|---|---|
| `Int` | `i64`; a refined `Int` is a newtype over it | the store carries at most 2^53; an argument beyond the word saturates to `i64::MAX`, which no world holds |
| Store key order | `version`, then states by name; a record's fields in declaration order | a module's states are a set, a record's fields a sequence; this reproduces the contract's bytes, though nothing in the spec states it |
| A `Map` in the store | the array of its values by key, only when every writer keeps the key in a field; the law `stored_under_its_number` now states the same | otherwise the backend refuses; the contract's format has no other map form |
| Formatting | `prettyplease`; `bin/check` does not run `cargo fmt --check` | the printer needs no toolchain at emit time; `rustfmt` would rewrap 14 of the files, layout only |
| Law start worlds | the initial world and every test used as an `@given` | the card allows any world the types and laws allow |
| Generated values | sums by variant; refined text from a fixed pool filtered by the claim; refined `Int` near its bound, sometimes far | a parameter's values come from its type; the pool is a runtime shim |
| Environment variable for the store | `<BIN>_STORE` | the contract names `TODO_STORE`; no vocabulary rule derives it (IR-NEEDS row 10) |
| The generated `Cargo.toml` | declares its own `[workspace]` | a standalone project never joins an enclosing workspace |

## Limits

- **One program.** The typed subset is what the todo program uses: no `var`, `while`, `fail` in a body, sums with data, consts, `Option` in the store. Each other form is an `E-UNSUPPORTED` diagnostic with its span and declaration, never a silent drop.
- **SIGINT during a write** can leave `<path>.<pid>.tmp` behind. The old store stays intact, because the replace is a rename. Removing the file on a signal needs a handler, and the generated crate forbids `unsafe` and has no dependencies.
- **Refusals are coarse.** A map read with no dominating `k in m` guard or key iteration, a claim no lower bound shows, and a store type with no JSON form are refused outright; a real planner would try more strategies first.
- **Timings are from a loaded machine** and one store shape; the ranges above are separate runs. Allocation counts and RSS are single runs; they repeat exactly for a fixed store, but the times do not.
- **Parity is 68 transcripts and the spec's tests and laws**, sampled. The generated laws caught one deliberate mutation of `clear`; that is not a mutation score.
- **Provenance is per item and per anchor**, not per expression.
