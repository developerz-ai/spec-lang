# C++ backend notes

Projected by hand by **Claude Opus 5.5** (`claude-opus-5-5`), standing in for `spec-lang build --target cpp`, from `src/todo/*.slg`, `src/apps/cpp.slg`, `docs/cli-vocabulary.md` and `docs/contract.md`.

## Result

| Check | Result |
|---|---|
| `bin/check` (clang-format `--dry-run --Werror`, build with `-Wall -Wextra -Wpedantic -Werror`, ctest) | passes, 8 of 8 test executables |
| `bun scripts/parity/run.ts --only cpp` | 60/60 cases |
| `bun scripts/parity/run.ts --only cpp --race 40` | numbers 1..40, none lost |
| Compilers | g++ 16.0.1 (the default `c++`) and clang++ 22.1.8 both build and pass ctest with `-Werror` |

## Layout

| Spec | Files |
|---|---|
| `todo.model` | `model/unicode` (UTF-8 validity, code points, the 25 White_Space code points), `model/text` (`is_title`, `is_tag`), `model/types.hpp` (`Title`, `Tag`, `TaskNo` with private constructors and `make`, `Status`, `Priority`, `Task`) |
| `todo.render` | `render/render` (`mark badge urgency rank` constexpr in the header; `tag_text line`) |
| `todo.board` | `board/world.hpp`, `board/errors.hpp`, `board/rules.hpp`; one source per rule (`add list done_reopen rename remove clear stats`) |
| `cli` vocabulary | `cli/params.hpp` (constexpr parameter table), `cli/help.hpp` (consteval help), `cli/command.hpp`, `cli/parser`, `cli/dispatch`, `cli/exit_code.hpp` |
| `@store` | `store/fd.hpp` (RAII), `store/json_lexer`, `store/decode`, `store/encode`, `store/lock`, `store/file` |
| `app TodoCpp` | `app/program` (parse, lock, load, rule, store, lines), `app/main.cpp` (argv, `TODO_STORE`, two writes) |

CMake targets enforce the purity rule: `todo_domain` (model, render, board) links nothing; `todo_store` and `todo_cli` link only the domain; `todo_app` joins them.

Tests (`tests/`, a 90-line harness, no framework): `model_cases` and `render_cases` (every `@cases`, plus code-point and White_Space edges), `scenarios` (the ten spec tests, same names), `laws` (400 seeded traces of 40 commands over all eight rules, splitmix64, every law in `laws.slg` after every step), `store_codec` (canonical bytes, round trip, 46 unreadable shapes), `store_files` (temporary directory: missing store, atomic replace, lock creation, contention), `cli_parser` (every usage error in the contract and parity), `program` (end to end in a temporary directory).

## Libraries

None. Considered and declined: `fmt` (std `to_chars` and string append cover every format here), doctest/Catch2 (a 90-line harness covers what the tests need, and a FetchContent download would make an offline build flake), simdjson (cannot reject duplicate keys or report integers above 2^53 the way section 3 requires without a second pass, and the store is small).

## Open choices: where the source did not determine the code

| Question | Chosen | Why |
|---|---|---|
| `--priority low --priority high` | `error: --priority given twice` | The contract says "given twice" for `Option` flags and "repeats" only for `Set`. `Priority = Normal` is neither. Last-wins would be a guess, so it is treated like an `Option`: one parameter, one value. **Other backends may differ; no parity case pins it.** |
| A title argument that is not valid UTF-8 | `<title>: not a Title` | `Text` is Unicode; bytes that are not UTF-8 are not a `Text`, so not a `Title`. |
| A store that is not valid UTF-8 (raw bytes, overlongs, encoded surrogates), or starts with a BOM | unreadable | RFC 8259 requires UTF-8 and lets a reader refuse a BOM; section 3 says "valid JSON". |
| `\u0000` or another escaped control in a stored title | unreadable | Decoded first, then `is_title` refuses bytes below 32. |
| `-0` as an integer | unreadable | It is an integer, but 0 is outside every range the store uses. |
| A `TaskNo` argument past 64 bits | saturates at 2^64 - 1 | `Int` is unbounded, but every guard that reads a number is `no in tasks`, and no store holds a number above 2^53, so any number above that answers `NoTask` exactly as the unbounded one would. Exactness is kept for every observable, not for the value. |
| Where 2^53 is enforced | in `encode`, as `Full` | The board stays exact (`next_no + 1` on 64 bits never overflows from 2^53); the store refuses a world it cannot hold. So `full` is reported after the rule ran, before anything is written. |
| `TODO_STORE` set but empty | treated as unset (`.todo.json`) | Otherwise the lock would be `.lock` and the store path `""`; refusing it would need a message the contract does not have. |
| A writing rule whose world did not change (`clear` with nothing done) on a missing store | writes the initial store | "A rule that writes and succeeds replaces the store". |
| The same, when the encoded bytes equal the bytes read | **not rewritten** | `docs/idea/32-build-layout.md`: a byte-identical file is not rewritten. The world on disk already is the new world, so the commit has happened. |
| Lock file open mode | `O_RDWR` for exclusive, `O_RDONLY` for shared, both `O_CREAT`, mode 0666 minus umask | `flock` works on a read-only descriptor; a reader needs no write access to an existing lock file. |
| Store file mode | 0666 minus umask on each replace; an existing file's mode is not carried over | Not specified; carrying it over needs an extra `fstat` and `fchmod`. |
| Directory `fsync` after `rename` | done; failure is `unwritable`; `EINVAL` (file system cannot sync a directory) is success | Makes "committed to disk" true across a crash. See "Could not keep". |
| Lock released before or after printing | before: `run` returns the text, `main` prints it after the lock's destructor ran | Lines go out "after commit"; holding the lock while a pipe blocks would stall other writers. |
| `-h` given as a flag's value (`add --tags -h`) | help | The contract's step 2 is lexical: "anywhere before the first `--`". |
| The first argument is `--` | `error: unknown command "--"` | Step 3: the first argument is the command. |
| Standard output cannot be written | `error: stdout: unwritable` on stderr, exit 1 | No code fits: the rule committed. SIGPIPE keeps its default, so `todo list \| head` ends quietly as Unix tools do. |
| A store that is not a regular file (a directory, a FIFO) | unreadable | Reading a FIFO would block under the lock. |
| `list` order when ranks tie | cannot happen | `rank` ends in the task number, which is unique. |
| `clear`'s reply and `add`'s reply | returned by the board functions, unused by the CLI | The CLI vocabulary maps only `out.line` to output. |

## Optimizations

Each one is licensed by a fact the spec states, which a compiler would know without rediscovering it.

| Trick | Fact that licenses it | Measured effect |
|---|---|---|
| Shared `flock` and no write path for `list`, `stats` | `@uses(tasks.read, out.write)` on both: no write effect | Not measured separately (concurrent readers do not wait on each other: `store_files` test `shared_readers_share`). |
| Read rules return lines, not a world | same read set; makes `reads_change_nothing` hold by type | none at run time; a law that cannot fail. |
| `stats` reads with a fused validator that keeps only two counters (no `Task`, no `std::map`, no title copies) | `stats` reads only `status` and `tasks.size`; the contract still requires the whole store be validated, so it is | 50 000 tasks, 5.4 MB store, hyperfine `-N -w 5 -r 40`, ext4: `stats` 14.3 ms ± 0.9, `list --all --tag nomatch` (full decode, prints one line) 31.1 ms ± 2.0. Same validation, so the gap is materialisation. |
| Schema-driven reader: no JSON tree; every unknown key or wrong value type rejects at once | section 3 fixes every key and type, so anything else is unreadable | not measured against a tree reader. |
| Zero-copy strings: a string without escapes is a `string_view` into the one read buffer; escaped strings decode into reused scratch buffers; tags are 32-byte inline values | `is_tag` bounds a tag to 32 ASCII bytes; titles are only kept by the full decode | not measured separately. |
| One `read` of the whole file, sized by `fstat` | the store is one small file read once per run | not measured. |
| UTF-8 validation of the whole buffer up front with an 8-byte ASCII fast path, then raw copy inside strings | section 3: JSON is UTF-8; titles are mostly ASCII | not measured. |
| ASCII fast path in `is_title`: one pass for bytes, blankness and size | `t.bytes.all(...)` is already a byte scan; for ASCII, `size` equals bytes and only ASCII white space can trim; U+0085, U+2028 and U+2029 are not ASCII, so the fast path is unchanged and the other path does three byte searches after UTF-8 validation | not measured. |
| Canonical writer: one reserved `std::string`, digits written in place by `to_chars`, one `write(2)` loop, `fsync`, `rename` | section 3 fixes the bytes; the world is in key order (`std::map`, `std::set`), so no sort at write | not measured. |
| A store whose new bytes equal the bytes read is not rewritten (no tmp, no `fsync`, no `rename`) | `docs/idea/32-build-layout.md`: byte-identical files are not rewritten | hyperfine `-N -r 50`, small store, ext4: `clear` with nothing done (skipped) 0.28 ms ± 0.04; `add x` (writes, two `fsync`s) 10.2 ms ± 0.3. Different rules; the gap is the write. |
| Parameter table `constexpr`; help text built at compile time by `consteval` from it into a `std::array<char>`; no static-initialisation allocation | the parameter list of each `cli` rule is fixed at build time | `--help` 0.28 ms ± 0.02 (process start dominated). |
| Release: `-O3`, LTO (`CMAKE_INTERPROCEDURAL_OPTIMIZATION`), `-static`, stripped | `@why("the board as one static binary")` in `src/apps/cpp.slg` | binary 961 432 bytes, statically linked; parity median 0.3 ms per run over 154 runs. |

Every number above is one run of hyperfine on one machine (x86-64, ext4); none is a benchmark suite.

## Could not keep

- **Exact `Int` for task numbers on the command line.** Kept for every observable, not for the value (see open choices).
- **"On an error the world is unchanged" if the directory `fsync` fails.** The `rename` has already happened, so the new store is visible while the run reports `unwritable`, exit 3. Linux reports this only on an I/O error.
- **Atomicity against a crash between `rename` and the directory `fsync`.** The store is then the old or the new one, never a mix; which one survives is up to the file system.
- **`Contended` timing.** 5 s is measured on the monotonic clock from the first try; the last try can land up to one interval (10 ms) later.

## Disagreements

None with the contract or the parity cases. One gap: the contract does not say what a repeated `--priority` does (see open choices); a parity case would pin it for all backends.

## Line counts

```text
find include src -name '*.?pp' | xargs wc -l | tail -1     # product: 2293
find tests -name '*.?pp' | xargs wc -l | tail -1           # tests:   1178
wc -l CMakeLists.txt bin/build bin/check | tail -1         # build:     85
```

Run from `out/cpp/`, after `clang-format`. 51 source files.

## Time

About 70 minutes wall clock, from the first read to a passing gate and parity.

## Limits

- The laws are sampled (400 seeded traces), not proven; `refusal_changes_nothing` and `reads_change_nothing` hold by the types as well.
- The race check covers 40 processes on one machine; it does not prove the lock correct.
- Measurements are single runs on one machine and one file system; they show an order of magnitude, not a speed-up to quote.
