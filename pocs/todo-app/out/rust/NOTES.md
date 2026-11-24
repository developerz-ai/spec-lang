# Rust projection: notes

> **Hand projection.** There is no `spec-lang build`. This crate is what one model session wrote by hand from `src/todo/*.slg`, `src/apps/rust.slg`, `docs/cli-vocabulary.md` and `docs/contract.md`, standing in for `spec-lang build --target rust`.

- **Projected by:** Claude Fable 5.2 (`claude-fable-5-2`), one session, `As of 2026-11-24`.
- **Toolchain:** `rustc 1.100.0`, `cargo 1.100.0`, edition 2024.
- **Wall-clock:** about 20 minutes from first read to green gate, by the session's own `date` calls (12:49 to 13:05 for code and tests, then these notes). Not timed with anything better.

## Result

| Check | Command | Result |
|---|---|---|
| Gate | `bin/check` (`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `bin/build`) | passes; 71 tests |
| Parity | `bun scripts/parity/run.ts --only rust` | 64/64 cases |
| Race | `bun scripts/parity/run.ts --only rust --race 40` | numbers 1..40, none lost |

## Layout

| Spec | Here |
|---|---|
| `todo.model` | `src/model/`: `title.rs`, `tag.rs`, `task_no.rs`, `status.rs`, `priority.rs`, `task.rs`, `text.rs` (`size`, `trim`) |
| `todo.render` | `src/render.rs` |
| `todo.board` | `src/board/`: `world.rs` (state), one file per rule, `command.rs` (events and dispatch), `out.rs` (`out.line`), `refusal.rs` |
| `cli` vocabulary | `src/cli/`: `params.rs` (table types), `commands.rs` (the table), `parse.rs`, `help.rs`, `usage.rs`, `exit.rs`, `run.rs` |
| `@store` | `src/store/`: `codec/` (`cursor.rs` strict JSON, `decode.rs`, `encode.rs`, `unreadable.rs`), `lock.rs`, `atomic.rs`, `file.rs`, `path.rs`, `error.rs` |
| `app TodoRust` | `src/app.rs` (store + board), `src/main.rs` (process edge only) |
| `todo.laws` | `tests/laws.rs` |
| `todo.tests.scenarios` | `tests/scenarios.rs` |
| `@cases` | `tests/model_cases.rs`, `tests/render_cases.rs` |
| the rest | `tests/store_codec.rs`, `tests/store_file.rs`, `tests/cli_parse.rs`, `tests/cli_help.rs`, `tests/cli_run.rs`, `tests/support/mod.rs` |

The domain (`model`, `render`, `board`) imports nothing from `std::fs`, `std::env`, `std::io` or `std::time`.

## Line counts

Counted `As of 2026-11-24` with `wc -l`, blank lines and comments included.

| What | Lines | Command |
|---|---|---|
| Product code | 2298 | `wc -l $(find src -name '*.rs') \| tail -1` |
| Tests | 1674 | `wc -l $(find tests -name '*.rs') \| tail -1` |
| Build files | 59 | `wc -l bin/build bin/check Cargo.toml clippy.toml .cargo/config.toml \| tail -1` |

No test code lives under `src/`, so the split is the two folders.

## Libraries

None. `[dependencies]` is empty, so there is nothing to pin.

| Would have been | Why not |
|---|---|
| `serde_json` | The reader must reject a duplicate key in any object, an integer literal above 2^53 without rounding, `1.0` and `1e0`, and a lone surrogate. `serde_json` keeps the last duplicate and reads numbers through `f64`/`u64` unless wrapped in custom visitors and `arbitrary_precision`; the wrapper would be about the size of the 214-line reader here, which also borrows strings from the input. |
| `thiserror` | Five small error enums; the hand-written `Display` impls are the contract's exact bytes and are short. |
| `tempfile`, `proptest` | The contract asks for an own seeded generator; `tests/support/mod.rs` has a 12-line SplitMix64 and a temporary directory under `CARGO_TARGET_TMPDIR`. |
| `clap` | Excluded by the contract. |

## Open choices

Places where the spec, the vocabulary and the contract did not determine the code. Four that this session first met as open (`--priority` twice, help as a flag value, `--` as a command, empty `TODO_STORE`) were settled by the contract while the work ran, the way this projection had already chosen; they are not listed.

| # | Left open | Chosen | Why |
|---|---|---|---|
| 1 | How an unbounded `Int` (`TaskNo`) is held | one `u64`; every argument literal above `u64::MAX` becomes one value, `TaskNo::BEYOND` | licensed by the store limit: no world holds a number at or above 2^53, and a rule only looks an argument up. Two different huge arguments compare equal, which no rule can observe. See Disagreements 1 |
| 2 | Where `full` is decided | in the projected rule: `board::add` returns `Err(Full)` when `next_no` is 2^53, before any write | the contract says "`add` when `next_no` is 2^53"; deciding it in the rule keeps "a refusal changes nothing" and makes the `u64` arithmetic total. `World::restore` refuses a `next_no` above 2^53, so the bound is a domain invariant here, though the spec has none |
| 3 | A rule that writes, succeeds and changes nothing (`clear` with nothing done, `rename` to the same title on a canonical store) | the store is not rewritten when the canonical bytes equal the bytes read | see Disagreements 2. A missing store is still created (`clear` on an empty directory writes the initial world), and a non-canonical store is still made canonical |
| 4 | An argument that is not valid UTF-8 | usage error `error: argument <n>: not UTF-8`, exit 2, `n` counted from 1 with the command as 1; `--help` before it still wins | every parameter type is `Text` or parsed from it; lossy conversion would silently store U+FFFD. The message is invented: the contract has none |
| 5 | Standard output cannot be written (closed pipe) after commit | the failure is dropped; exit code stays what the rule decided (0) | the world is committed and there is nobody to tell. The contract has four exit codes and none for this |
| 6 | `--tag a --tag` (seen before and no value left) | `--tag needs a value` | the contract lists "none left" before "seen before"; I took the list order as the check order |
| 7 | How long a reader holds the shared lock | until the bytes are read; decoding and printing happen after release | the contract's "from before the read until after the write" has no write for `list` and `stats` |
| 8 | Lock errors other than "would block" (`flock` fails, the lock path is a directory) | `unwritable` | "the store or its lock cannot be created" was the nearest row |
| 9 | Store exists but cannot be read (permission, it is a directory) | `unreadable` | "cannot be read" |
| 10 | Directory `fsync` after `rename` | not done | the contract names write, `fsync`, `rename`. After a power loss the rename itself may be lost; the store is then the old one, never a torn one |
| 11 | Mode of a new store and lock file | process default (`0666` minus umask); an existing store's mode and owner are not carried over the rename | not mentioned |
| 12 | A stale `<path>.<pid>.tmp` from a killed run | left; a failed write removes its own | not mentioned |
| 13 | When a usage error and a store error both apply | usage first; help and usage errors never open the store or create the lock file | parity `= absent` cases imply it for the store, not for the lock file |
| 14 | Depth and size of a store | no limit needed: the reader is schema-directed and never recurses past `tasks[i].tags[j]`; the file is read whole into memory | |
| 15 | Rule replies (`add -> TaskNo`, `clear -> Int`) | returned by the rule functions, dropped at the command line | the vocabulary prints only `out.line` |
| 16 | `out.text` across steps of one test | one `Out` per test, appended by each call | the scenarios need it |
| 17 | `others_untouched` quantifies over every `TaskNo` `k` | checked for every `k` present before or after the step | for any other `k` both sides are `none` |
| 18 | Where a law's trace starts | two thirds of traces from the initial world, one third from `three_open_one_done` | the card allows "any world its types and laws allow"; no generator for arbitrary worlds was written |
| 19 | `unknown_numbers_are_refused(no: TaskNo)` generation | 1..=12, 2^53, `BEYOND`, and 300 seeded 64-bit values | a property needs a generator; the spec gives the type only |
| 20 | Law tests count | 240 traces x 40 commands per law, same seeds for each law, reads included in the traces | contract minimum is 200 |
| 21 | Static linking | `+crt-static` for `*-unknown-linux-gnu` in `.cargo/config.toml` | the app's `@why` says "one static binary"; a `@why` is a reason, not a claim, so this is a reading |
| 22 | `Halt::Full` is not a spec error name | printed as the contract's `store <path>: full`, exit 3 | |

## Optimizations

Each is licensed by a fact the source states. Measurements: `As of 2026-11-24`, one 16-thread Linux machine, `hyperfine -N` (30 runs unless said), release build, a generated store of 100 000 open tasks (13.6 MB, two tags and a 50-byte title each). One machine, one store shape, warm page cache: read them as direction, not as numbers to quote.

| Trick | Fact that licenses it | Measured |
|---|---|---|
| `list` and `stats` take the shared lock and can reach no code that writes the store (`Store::read`, `Store::fold` vs `Store::update`) | `@uses(tasks.read, out.write)` on both rules | not measured |
| `stats` folds over the stored tasks and builds no `Task`, no map, no owned title or tag (`Store::fold` + `board::Tally`); the store is still validated in full | the body of `stats` reads only `tasks.size` and each `status` | `stats` 19.0 ms ± 1.2 against `list --tag zzz` 45.2 ms ± 1.6 on the same store (the second prints one line but builds the world). `tests/store_codec.rs::the_fold_sees_what_the_world_holds` checks the fold against the rule on 300 worlds |
| Schema-directed reader: no JSON value tree; a string without escapes is a slice of the input; unknown shapes fail where they stand, so there is no skip-a-value code | the "Accepted on read" table is closed: exactly these keys, these types | not measured separately; it is what the row above runs on |
| Integers are read exactly in a `u64` with the bound checked per digit | "an integer, 1 to 2^53"; "must be rejected, not rounded" | not measured |
| Uniqueness of task numbers and of tags costs one comparison per item when the file is in canonical order; a sort only otherwise | "tasks by `no` ascending, tags in code-point order" is what every backend writes | not measured |
| Canonical writer: one buffer sized once from an upper bound, one `write`, `fsync`, `rename`; nothing is sorted | key order and task order are fixed by the contract; `Map` and `Set` are held as `BTreeMap`/`BTreeSet`, which iterate in those orders | not measured |
| Tags and titles are written with no escape scan beyond `"` and `\` (tags with none at all) | `is_tag` admits only `a-z 0-9 -`; `is_title` admits no byte below 32 | not measured |
| A write whose bytes equal the bytes read is skipped | see Disagreements 2 | `rename 5 x` when it changes the title 66.7 ms ± 2.5; when the title is already `x` 57.2 ms ± 2.5 |
| `TaskNo` is one `u64`, `Copy` | `next_no` at most 2^53, numbers below `next_no` (contract section 3; law `numbers_below_next`) | not measured |
| `done`, `reopen`, `rename`, `remove`: both guards and the write use one map lookup | guards are pure reads of `tasks[no]` and run before the body | not measured |
| `clear` never builds `gone`; one `retain` and a size difference | `gone` is used only for `gone.size` and as the set removed | not measured |
| `list` sorts references with an unstable sort; lines format straight into the output buffer | `rank` ends in `t.no`, which is unique, so the order has one answer; `line` is pure | not measured |
| `tag_text` does not sort | `Set[Tag]` is held ordered; `tags.sort(it)` is then the identity | not measured |
| `is_title` fused into one or two passes with an ASCII path and an 800-byte early exit | the clauses are pure; U+0085, U+2028 and U+2029 are not ASCII, so only the non-ASCII pass looks for them; the byte clause already excludes every ASCII white space but U+0020; a code point is at most 4 bytes and the size limit is 200. `tests/model_cases.rs::is_title_is_the_clauses` checks it against the expression written clause by clause on 5000 strings | not measured |
| `is_tag` checks byte length first, never counts code points, and reads `starts_with("-")` as a test of the first byte | every accepted byte is ASCII | not measured |
| Command table is a `static`; help is a `const` string | `@on(cli(...))` rules and their parameters are fixed at build time. `tests/cli_help.rs` derives the help from the table again and compares | not measured |
| Arguments move into values: a title from the command line is never copied; the event moves into the rule | each parameter is used once | not measured |
| All reply lines go out in one `write` after commit | `out.line` is "sent after commit", so nothing needs them earlier | not measured |
| Usage errors and help never touch the store or the lock | "a claim fails at the edge: the rule never ran" | not measured |
| Release profile `opt-level = 3`, `lto = "fat"`, `codegen-units = 1`, `panic = "abort"`, `strip = true`; static C runtime | the app's `@why("the board as one static binary")`; no code path unwinds on purpose (`clippy::unwrap_used`, `expect_used`, `panic` are denied outside tests, `unsafe` is forbidden) | static 1 282 352 bytes, 37 syscalls and 264 µs ± 35 for `stats` in an empty directory; the same source linked dynamically 406 448 bytes, 73 syscalls, 499 µs ± 75 (`ls -l`, `strace -c`, `hyperfine -N -r 300`). Static is larger on disk and starts faster |

Not done, though the source would license it: `list` could filter while decoding and build only the tasks it shows (its filter reads `status` and `tags` only). `decode` checks each title and tag a second time when it builds the owned `Title` and `Tag` after `scan` has checked the borrowed text; that keeps the claim checked in one constructor, at the price of one more pass over each string on the write path and for `list`.

## Could not keep

- **"`Int` is exact"**, in full. Kept for everything a user can see; not kept inside the type (Open choice 1).
- **The lock is exclusive "from before the read until after the write"** is kept. That the *print* comes after commit is kept. Nothing checks that a crash between `rename` and the print loses only the print; it follows from the order of the code.
- Nothing else in the contract is knowingly unmet. `flock` comes from `std::fs::File::try_lock` and `try_lock_shared`, which are `flock(2)` on Linux; on another Unix the standard library may choose differently, and this was run on Linux only.

## Disagreements

Implemented as the contract says, except 2, where the coordinator relayed that skipping is allowed.

1. **A `TaskNo` beyond a machine word.** The vocabulary says such an argument is "a `TaskNo`, so `NoTask`", and the contract says `Int` is exact. Both are met from outside, but only because the 2^53 store limit, which the contract calls "a backend constraint, not a language rule", guarantees the number is absent. So a backend constraint is what makes a language rule cheap to keep. A compiler would need that limit as a stated claim on `next_no` (for example `type TaskNo = 1..=2^53`) to choose `u64` honestly; here it was read out of the contract's prose.
2. **Skipping a byte-identical write.** Contract section 3 says a rule that writes and succeeds "replaces the store atomically". This projection does not replace it when the bytes would be the same. `docs/idea/32-build-layout.md` has that rule for build outputs, not for a program's store; applying it here is a stretch that I was told is acceptable. It is invisible to the parity suite and visible to anything watching the inode or the modification time. If the contract means "always replace", delete the comparison in `src/store/file.rs::update`.
3. **`full` is exit 3 with a store message, but it behaves as a refusal** (world unchanged, nothing printed, decided before any write). It reads more naturally as `error: Full`, exit 1, like `Contended`. Implemented as the contract has it.
4. **`Contended` is exit 1 "a rule refused"**, though no rule ran, while a claim failing at the edge is exit 2 for exactly that reason. Implemented as the contract has it.
5. **The vocabulary's derivation table has no row for a defaulted parameter that is not `Bool`, `Option` or `Set`** (`priority: Priority = Normal`). The contract now covers it; the vocabulary card still does not, so the flag `--priority` is derived from the contract's help text and not from the card.
6. **Help text shape for a value is not derivable from the card**: `low|normal|high` for a sum type but `<tag>` for `Tag`. `tests/cli_help.rs` encodes "a sum type shows its variants, anything else `<type name in lower case>`", which fits the eight commands and is a guess beyond them.

## Limits

- One session, one machine, Linux x86-64 only. The static-link flag is set for two Linux GNU targets and nothing else was built.
- The law tests sample: 240 seeded traces of 40 commands. They are evidence at the `sampled` level, not a proof, and the traces start from two worlds only.
- The measurements are from one machine, one synthetic store and warm caches; nothing in the table was compared against a version without the trick except where the row names both sides.
- The concurrency evidence is the parity race (40 processes) and one in-process test (8 threads x 10 adds). Neither proves the lock correct, and a store on a network file system is not covered.
- "Zero dependencies" means the JSON reader is 214 lines that only this crate's tests have exercised.
- The wall-clock figure is a reading of `date` output at a few points, not a timer.
