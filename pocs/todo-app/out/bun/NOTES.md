# out/bun: notes

Hand projection of `src/todo/*.slg` + `src/apps/bun.slg` to a Bun CLI. It stands in for `spec-lang build --target bun`. Paths below are relative to the POC root (`pocs/todo-app/`) unless they start with `out/bun/`.

- **Projected by:** Claude Opus 5.5 (`claude-opus-5-5`), one session, as the "TypeScript/Bun backend".
- **Toolchain:** Bun 1.4.5, TypeScript 5.9.3 (strict, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes`), Biome 2.3.8.
- **Gate:** `out/bun/bin/check` (biome check, `tsc --noEmit`, `bun test`, `bin/build`). It passes: 203 tests, 0 failures.
- **Parity:** `bun scripts/parity/run.ts --only bun` gives 64/64. `--race 40` passes, and so did `--race 200` (one run).
- **Wall-clock:** about 25 minutes from the first read to this file, including the benchmarks.

## Layout

| Spec | Folder | Files |
|---|---|---|
| `todo.model` | `src/model/` | `text.ts` (size, trim, bytes), `title.ts`, `tag.ts`, `task-no.ts`, `status.ts`, `priority.ts`, `task.ts` |
| `todo.render` | `src/render/` | `mark.ts`, `badge.ts`, `tag-text.ts`, `rank.ts`, `line.ts` |
| `todo.board` | `src/board/` | `world.ts`, `outcome.ts`, one file per rule (`done.ts` holds done and reopen) |
| `cli` vocabulary | `src/cli/` | `arg-types.ts`, `params.ts`, `commands.ts` (the table), `parse.ts`, `help.ts`, `run.ts`, `execute.ts`, `output.ts`, `exit-codes.ts`, `store-path.ts` |
| `@store` | `src/store/` | `json-reader.ts`, `decode.ts`, `canonical-read.ts`, `encode.ts`, `read-store.ts`, `libc.ts`, `lock.ts`, `atomic-write.ts`, `session.ts`, `limits.ts`, `store-error.ts`, `system-error.ts` |
| `app TodoBun` | `src/main.ts` | reads argv and env, writes fd 1 and 2, exits |
| `@cases` | `test/model/`, `test/render/` | `test.each` tables |
| `todo.tests.scenarios` | `test/scenarios.test.ts` | one `test` per spec test, with the same names |
| `todo.laws` | `test/laws/` | fast-check, seed 20261124, 200 traces × 40 events, each law checked after every step |
| store, cli | `test/store/`, `test/cli/` | codec, strict reader, canonical fast path, session in a temp dir, parser table, help bytes, end to end |

Domain refusals are values: `Outcome<R> = {ok: true, world, lines, reply} | {ok: false, refusal: "NoTask" | "AlreadyDone" | "NotDone"}`. The store returns `{ok: false, error: "unreadable" | "unwritable" | "full" | "contended"}`. Nothing throws for a refusal. `throw` is kept for bugs: a write under a shared lock, or a non-OS error inside a `catch`.

## Libraries

| Package | Version | Kind | Replaces |
|---|---|---|---|
| `fast-check` | 4.10.2 | dev | a hand-written seeded PRNG and generators for the law traces, the `unknown_numbers_are_refused` property and the differential tests. It also shrinks a failing trace. |
| `typescript`, `@types/bun`, `@biomejs/biome` | 5.9.3, 1.3.3, 2.3.8 | dev | (toolchain) |

There are no runtime dependencies. `trustedDependencies` is `[]`. flock comes from libc through `bun:ffi`. The JSON reader is written by hand because `JSON.parse` accepts duplicate keys and rounds integers. The argument parser is written by hand because contract section 2 fixes its bytes.

## Open choices: where the source did not decide the code

Each row had more than one reasonable behaviour.

| # | Question | Chose | Why / where |
|---|---|---|---|
| 1 | How a `Text` holding a lone surrogate counts (`Text` is scalar values, but a JS string can hold one) | It is not a Title or Tag: `isWellFormed()` is checked first | Without that check, `TextEncoder` would turn it into U+FFFD and it would pass. `src/model/title.ts` |
| 2 | Invalid UTF-8 in argv | Whatever Bun decodes (U+FFFD), then checked as text | No parity case covers it. A projection that could see raw argv bytes would refuse them. Not tested. |
| 3 | `is_title` is defined on UTF-8 bytes | Literal on bytes, with an ASCII fast path that a differential property test checks against the literal form | C1 controls (U+0080..U+009F) encode as bytes ≥ 0x80, so the byte clause passes them; U+0085, U+2028 and U+2029 are refused by the three `contains` clauses, checked on the slow path only |
| 4 | `Set[Tag]` representation | A frozen array, sorted and without repeats | Then `==` is element-wise, and the writer and `tag_text` need no sort |
| 5 | `Int` / `TaskNo` representation | `bigint` everywhere, never `number` | Arguments of any length work, and the 2^53 checks are exact |
| 6 | Repeating a defaulted non-Option flag (`--priority`) | `--priority given twice` | The vocabulary only says this for `Option`. The contract has since been updated to say any non-Set value flag. |
| 7 | When a repeated value flag has no value left (`--tag a --tag`) | `needs a value` is checked before `given twice` | That is the order the contract lists them in. No parity case covers it. |
| 8 | `-h` inside a flag's value or before a `--` that is itself a value (`add --priority -- -h`) | Literal: scan argv up to the first argument equal to `--`. So `add x --priority -h` is help, and `add --priority -- -h` is `--priority: not a Priority` | Contract step 2 is read literally. The updated contract confirms the first case. |
| 9 | Empty `TODO_STORE` | Counts as unset | First a choice here, now in the contract |
| 10 | A write rule whose new world encodes to the same bytes as the file (e.g. `clear` with nothing done) | Not rewritten | `docs/idea/32-build-layout.md` says byte-identical files are not rewritten. A non-canonical store is still rewritten canonically after any write rule. |
| 11 | Whether a write rule that changed nothing still writes | It writes when the bytes differ (row 10) | So any successful write leaves a canonical store |
| 12 | Directory fsync after `rename` | Not done | The contract lists write, fsync, rename. A failing dir fsync after the rename would report an error after the commit was already visible. So durability of the rename across power loss is the file system's. |
| 13 | Byte-order mark at the start of the store | Unreadable | RFC 8259 forbids it. `TextDecoder` is built with `ignoreBOM: true` so the reader sees it. |
| 14 | JSON nesting depth | Over 64 is unreadable | Bounds recursion. Every valid store has depth 3. |
| 15 | Store path is a directory, or reading fails with an error other than ENOENT | `unreadable` | "cannot be read" |
| 16 | Lock file cannot be opened | `unwritable`, for `list` and `stats` too | Matches the `TODO_STORE=lists/home.json` parity case |
| 17 | `flock` fails with an error other than EWOULDBLOCK or EINTR | `unwritable`. EINTR is retried at once | Not specified |
| 18 | Waiting for the lock | `performance.now()` deadline of 5 s, `Bun.sleepSync(10)` between tries | Synchronous, since a rule never waits on the outside world |
| 19 | Help text | Derived from the command table (`p.usage` per parameter), and a test pins it to the contract bytes | So the vocabulary's "every cli rule gives a line of --help" is real |
| 20 | Platform | Linux glibc only: `dlopen("libc.so.6")`, Linux `EWOULDBLOCK = 11` | macOS and musl would need another library name and errno values |
| 21 | File modes | Lock and tmp opened `0o666`, reduced by the umask | Not specified |
| 22 | `rank` as a tuple | Projected as a comparator `compareRank` | Same order. No tuple is allocated. |
| 23 | Stdout and stderr writes | `fs.writeSync` loops on fds 1 and 2, then `process.exit` | So no buffered output is lost on exit |

## Optimizations

Each one is licensed by a fact the spec or contract states. Times come from `hyperfine -N --warmup 20 --runs 200..300` on this machine (Linux, NVMe), run under `heavy`, with the other agents' load present. They are one machine on one day; treat them as indications.

| Trick | Fact that licenses it | Measured effect |
|---|---|---|
| `--no-compile-autoload-dotenv --no-compile-autoload-bunfig` | `@store` and `TODO_STORE` are the program's only inputs besides argv. No config file is part of the world. | **Large.** By default the binary lists every ancestor directory looking for `.env` and `bunfig.toml`. strace showed 553 `getdents64` calls from a cwd under a crowded `/tmp`. `--help` went from 49.6 ± 2.2 ms to 5.4 ± 0.4 ms. It depends on the cwd: it was negligible inside the repo. |
| `--bytecode` | The program is fixed at build time | `--help` 5.3 → 3.6 ms. `stats` on 1000 tasks 11.6 → 9.1 ms. |
| `--minify` | (none needed) | Within noise: 5.35 vs 5.28 ms. Kept for size. |
| Shared lock, no writer call, store never rewritten for `list`/`stats` | `@uses(tasks.read, out.write)` on list and stats | Not measured separately. It is the contract's own rule. |
| Write skipped when the canonical bytes equal the bytes read | Rules are pure functions of the world; `docs/idea/32-build-layout.md` | Saves a tmp write, an fsync and a rename. Not timed. |
| Canonical fast-path reader (`src/store/canonical-read.ts`): one sticky regex per task, no JSON tree. Anything it does not recognise falls through to the general strict reader, which alone decides `unreadable`. | Contract section 3 fixes the bytes, and this program is their writer. Ascending `no` and ascending tags (what the writer emits) make repeats impossible. | `stats` on 1000 tasks 9.02 → 6.27 ms (median). `list` 9.55 → 6.99 ms. `rename` 10.0 → 7.3 ms. A 300-run differential property test checks that it equals the general reader on every store the writer can produce. |
| Sticky-regex scanning in the general reader (white space and plain string runs), and "already strictly ascending" tags in the decoder | JSON grammar; the writer sorts tags | End to end within noise (9.0 vs 8.8 ms). Kept because it is no more code than the per-character loop. |
| ASCII fast path in `is_title` (`/^[\x20-\x7e]*$/`, then length and "not all spaces") | In printable ASCII, a byte, a code point and a UTF-16 unit are the same thing, and U+0020 is the only White_Space; the three refused line breaks are outside it, so the fast path needs no check for them | Not timed end to end. A 2000-run differential test checks it against the literal claim. |
| `is_tag` as one regex `/^[a-z0-9][a-z0-9-]{0,31}$/` | Every allowed byte is ASCII, so `starts_with("-")` is a test of the first unit. The canonical reader's `TASK` regex repeats this tag pattern and changes with `is_tag` | Not timed. A differential test runs it against the literal claim. |
| `dlopen` of libc on the first lock only | Help and usage errors use no store (contract section 2 parsing comes before the store) | Not separable from startup |
| Frozen command table and help string built once at load | The command set is fixed by `app TodoBun(rules: [...])` | Not timed |

**Considered and not done:**
- Lazy `import()` of the store and writer modules on the read path. A compiled empty Bun program (`writeSync(1, "hi\n")`, same flags) starts in 3.03 ± 0.24 ms. `todo --help` takes 3.66 ± 0.32 ms and `todo stats` on an empty store 3.92 ± 0.29 ms. That leaves at most about 0.6 ms for all module evaluation, and `--bytecode` builds CJS, where top-level `await` is not allowed. Not worth a second entry point.
- A `stats`-only fused read that builds no `Task` objects. The contract requires full validation, and the canonical path already costs about 6 µs per task. The gain would be part of 2.5 ms at 1000 tasks, for a second decoder.

## Could not keep / limits

- `refusal_changes_nothing` and `reads_change_nothing` are true by type in the pure projection: a refusal carries no world, and list and stats return the world they got. The pure law test therefore also checks that no rule mutates its input world. `test/cli/run.test.ts` checks both laws again on the **bytes of the store file** across 25 random 20-command CLI traces.
- `unknown_numbers_are_refused(no: TaskNo)` draws from 1..2^80, not from all of `Int`.
- `others_untouched` quantifies over every `k`. The test checks every key before or after the step, each key + 1, both `next_no`s and 1.
- Errno is read through `__errno_location` right after the FFI call. If the runtime made a syscall in between, the value could be wrong. A wrong value can only turn "contended" into "unwritable", or the reverse.
- Linux glibc only (open choice 20).
- The race check shows uniqueness under 40 (and once 200) concurrent processes. It does not prove the lock correct.

## Disagreements

None that needed the contract overridden. Close calls:
- The contract's "full" (`add` when `next_no` is 2^53) is not a guard in `board.add`. It is carried in the store encoder, so the pure `add` stays unbounded as the spec says. Only the store refuses.
- `@cases` on `tag_text` writes `Set("work", "home")`, and `tags.sort(it)` sorts in code-point order. For tags (all ASCII) that equals JS's default sort, so no special comparator is projected. If `Tag` ever allowed non-ASCII, `tagSet` would need one.

## Line counts

```text
cd out/bun
wc -l $(find src -name '*.ts') | tail -1    # product: 1307
wc -l $(find test -name '*.ts') | tail -1   # tests:   1119
wc -l bin/*                     | tail -1   # scripts: 16
```

44 product files and 19 test files. Counts include header comments and blank lines.
