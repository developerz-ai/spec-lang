# Ruby projection: notes

> **Hand projection.** There is no spec-lang compiler. Every file in this folder was written by a model standing in for `spec-lang build --target ruby`, from `src/`, `docs/cli-vocabulary.md` and `docs/contract.md`.

| | |
|---|---|
| Projected by | Claude Fable 5.2 (`claude-fable-5-2`), one session |
| Toolchain | Ruby 4.0.10; runtime needs the standard library only (`strscan`) |
| Gate | `bin/check`: rubocop, `ruby -wc` on every file, minitest under `-w` (any warning fails), `bin/build` |
| State `As of 2026-11-24` | `bin/check` passes; parity 65/65 alone and 65/65 in `--mix`; `--race 40` passes |
| Wall clock | about 20 minutes from first read to green gate, by a start stamp; not a controlled measurement |

## Layout

| Spec | Files |
|---|---|
| `todo.model` | `lib/todo/model/`: `text` (White_Space, is-a-Text), `title` (`is_title`), `tag` (`is_tag`), `task_no`, `status`, `priority`, `task` (validated `Data`), `invalid` |
| `todo.render` | `lib/todo/render.rb` |
| `todo.board` | `lib/todo/board/`: `world`, `counts`, `outcome`, `refusals`, one file per rule |
| `cli` vocabulary | `lib/todo/cli.rb`, `lib/todo/cli/`: `types`, `command`, `commands` (the table), `parser`, `help`, `dispatch`, `exit_code`, `usage_error` |
| `@store` | `lib/todo/store/`: `json_reader`, `decoder`, `encoder`, `lock`, `atomic_file`, `file_store`, `errors` |
| `app TodoRuby` | `lib/todo/app.rb`, `exe/todo` |
| `todo.laws` | `test/board/laws_test.rb` with `test/support/traces.rb`: 250 seeded traces of 40 commands, every law after every step |
| `todo.tests.scenarios` | `test/board/scenarios_test.rb`: one test per spec test, same names |
| `@cases` | `test/model/title_test.rb`, `test/model/tag_test.rb`, `test/render_test.rb` |

## Libraries

All development only. `exe/todo` runs with RubyGems disabled and loads none of them.

| Gem | Pinned | Replaces |
|---|---|---|
| `minitest` | 6.0.0 | a hand-written test runner (it ships with Ruby as a bundled gem; pinned so the gate is the same everywhere) |
| `rubocop` | 1.92.3 | hand review of style and complexity; the lint half of the gate |
| `rubocop-minitest` | 0.41.0 | hand review of the tests' assertions |

Transitive versions are in `Gemfile.lock`. Gems install into `vendor/bundle` (`.bundle/config`), never into the machine's gem directory. Not used on purpose: `json` (see Optimizations), `optparse` (the parser's bytes are fixed by contract section 2).

## Choices the source left open

Each row is a place where `src/` plus the contract did not determine the code. Rows marked **settled** were open when first projected and were later fixed by a contract update that matched the choice made here.

### Language to Ruby

| Question | Chosen |
|---|---|
| What is a rule in Ruby? | a module function `Board.<rule>(world, **params) -> Outcome(world, lines, reply)`; every parameter a keyword, so dispatch from the table is one `public_send` |
| How is a refusal carried? | an exception: `Board::Refusal` with subclasses `NoTask`, `AlreadyDone`, `NotDone`. Raised before any new world exists. The printed name is the class name |
| Are `Title` and `Tag` their own classes? | no. They are frozen UTF-8 `String`s; the claim is checked by `Model.title?` / `Model.tag?` in the `Task` constructor (and again by `with`). A wrapper class per `Text where` type would be the stricter projection |
| Sum types with no data | symbols named as the variant: `:Open`, `:Done`, `:Low`, `:Normal`, `:High`. The store name is `Symbol#name`, the flag value is its lower case |
| `Option[Tag]`, `none` | `nil` |
| `Map`, `Set`, record | frozen `Hash`, frozen `Set`, `Data.define`. Equality is structural for all three, as the card requires |
| The two `state`s | one immutable `Board::World(tasks, next_no)`; a rule returns a new one. Atomicity inside the process is then free |
| A constructor given a value outside its type | `Model::Invalid` (an `ArgumentError`): a programming error, not mapped to an exit code, because every edge checks first |
| `Text` that is not Unicode (a Ruby string with invalid UTF-8) | not a `Text`, so not a `Title`, `Tag` or `TaskNo`: a usage error from argv, `unreadable` from the store |
| Encoding of argv | always read as UTF-8, whatever the locale says (`LANG=C` included) |
| `Text.bytes` | the UTF-8 bytes. The card does not say; the contract's code-point rule and the parity cases imply it |
| `tags.sort(it)` | byte order, which is code-point order for UTF-8 |

### Command line

| Question | Chosen |
|---|---|
| A plain-default value flag given twice (`--priority low --priority high`) | `error: --priority given twice`. **settled** |
| `-h` as a flag's value (`add x --tags -h`) | help wins. **settled** |
| First argument `--` or a flag | it is the command, so `unknown command "--"`. **settled** |
| `TODO_STORE=""` | counts as unset. **settled** |
| `--tag a --tag` (seen before, and no value left) | `--tag needs a value`: the contract lists "needs a value" before "given twice", and that order is kept. No parity case |
| `--tag a --tag B` (seen before, and a bad value) | `--tag given twice`, same reasoning. No parity case |
| A value flag followed by `--` or by another flag | consumed as the value, then checked against its type. No `Tag` starts with `-`, so `list --tag --all` is `--tag: not a Tag` |
| Does a usage error touch the store? | no file is opened and no lock file is created |
| When are lines printed relative to the lock? | after the lock is released, so a blocked reader of our output never holds the lock |
| Output cannot be written (reader gone) | `SIGPIPE` keeps its system default: the process dies quietly, as other tools do. Any other write error on standard output is not handled and ends in Ruby's uncaught-exception report; the contract has no exit code for it |
| Help text | derived from the parameter table at load, and compared with the contract's bytes in a test |

### Store

| Question | Chosen |
|---|---|
| Where is `full` decided? | in the encoder: a world whose `next_no` is above 2^53 cannot be written. The rule runs first (the board is unbounded, as `Int` is), the store refuses the result, nothing is written or printed |
| A write rule that changes nothing | if the canonical bytes equal the bytes read, the replace is skipped (see Optimizations). If the store is missing it is created: `clear` in an empty directory writes the initial world |
| Read failure other than "missing" (`EACCES`, a directory, `ENOTDIR`) | `unreadable` |
| `flock` fails other than "would block" | `unwritable` |
| Lock file open mode | `O_RDONLY | O_CREAT`, mode 0644: `flock` needs no write access, so a reader can lock a lock file it may only read |
| Store file mode and identity | mode 0644 less the umask; the old file's mode and owner are not carried over, and a store that is a symbolic link is replaced by a regular file |
| `fsync` of the directory after `rename` | not done; the contract asks only for the file. A crash right after the rename can lose the rename on some file systems |
| Temporary file after a failed write | removed, best effort. One left by a killed process is not cleaned up; the next write by the same pid truncates it |
| Lock wait | monotonic clock; try, then give up if 5 s have passed, else sleep 10 ms. No extra attempt after the deadline |
| JSON nesting | values nested deeper than 16 are refused. The format nests 4 deep, so nothing readable is lost; the bound keeps hostile input off the stack |
| Lone surrogate, byte order mark | refused by the reader itself, anywhere in the file |
| "No key twice", "no tag twice" | compared after unescaping: `"a"` and `"a"` are the same key and the same tag |
| Duplicate task `no` | checked on the number, wherever the tasks are in the array |

### Tests

| Question | Chosen |
|---|---|
| Where does a trace start? | half from the initial world, half from a random world the types and `numbers_below_next` allow (card, how it runs, 2) |
| What is in a trace? | all eight rules, weighted towards `add`; a named `no` is an existing task four times in five, otherwise random up to `next_no + 3`, so refusals happen (the test fails if they do not) |
| `others_untouched(t, k)`: which `k`? | every key in the before or after world, plus two numbers neither holds |
| `refusal_changes_nothing`, `reads_change_nothing` | true by construction here (immutable values, a refusal raises before a new world exists). The test still compares, and also checks a deep copy of the world before each step to catch a change in place |
| `unknown_numbers_are_refused(no: TaskNo)` | 207 values: the edges 1, 4, 5, 6, then 2^53, 2^64+1, 10^40, then 200 seeded draws up to 10^30. Not shrunk |
| `test three_open_one_done = {}` | a test that the fixture is the world the `@given` describes |
| Seed | `TODO_SEED`, default 20261124; trace `i` uses `seed + i`, and a failure names it |
| `is_title`, `is_tag` beyond `@cases` | a word-for-word transcription of the spec formulas (`test/support/spec_oracle.rb`) is the oracle for the optimized product code, over 2,000 seeded texts each |

## Optimizations

Method for every number: Ruby 4.0.10 on one 16-core Linux machine, interpreter binary called directly (no version-manager shim). "Process" rows are the median of 101 spawns; "in-process" rows the median of 21 calls. One machine, one run of the benchmark, no confidence intervals. A row without a number was not measured.

| Trick | Fact that licenses it | Measured |
|---|---|---|
| `#!/usr/bin/env -S ruby --disable-gems` | the projection uses the standard library only, so RubyGems is never needed at run time | `todo --help`: 37.6 ms with RubyGems, 7.8 ms without (process) |
| Load only what the command needs: help and usage errors load the parameter table alone; `dispatch` loads one rule file | parsing is derived from the rule's parameters, not from its body; a usage error means no rule runs | `--help`: 11.7 ms loading all of `lib/todo`, 7.8 ms lazy. `stats` in an empty directory: 11.7 ms eager, 9.9 ms lazy (process) |
| Read-only path for `list` and `stats`: shared lock, encoder and atomic writer never loaded, store never rewritten | `@uses(tasks.read, out.write)`: no `tasks.write`, no `next_no.write` | included in the row above |
| `stats` reads `Counts(open, size)` instead of the world: every check still runs, no `Task`, `Set` or map is built | the body of `stats` reads only `tasks.size` and each `status`; the contract still requires the whole store to be valid | 10,000 tasks, 1.04 MB: decode 78.8 ms as counts, 115.6 ms as world (in-process); `todo stats` 103.6 ms against 138.7 ms for a command that reads the world (process). 20 tasks, 2 KB: 0.17 ms against 0.23 ms per read. The JSON tree is still built in both; only the model objects are saved |
| The claims on title and tags are checked once on the world path (by the `Task` constructor), not once by the decoder and again by the constructor | `type Title = Text where is_title(self)`: the constructor is the check | not measured apart |
| A write whose bytes equal the bytes read is skipped | canonical bytes are a function of the world (contract section 3), so equal bytes mean an equal world; `docs/idea/32-build-layout.md`: a byte-identical file is not rewritten. Seen from outside only the file's inode and time differ | saves one write, `fsync` and `rename`: 5.6 ms for the 1.04 MB store on this disk (in-process). Costs one string comparison |
| Canonical writer: one buffer sized up front, one `write`, `fsync`, `rename` | the format is one line with a fixed key order | 12.9 ms to encode 10,000 tasks (in-process); not compared with another writer |
| Tags are written with no escaping pass | `is_tag`: only `a-z`, `0-9`, `-` | not measured |
| Titles are escaped only when they hold `"` or `\`, found by one scan | `is_title`: no byte below 32, so nothing else in a title needs a JSON escape | not measured |
| `is_title` without a trimmed copy or a byte array: byte length first, then two scans | `t.trim().size >= 1` holds exactly when one code point is not White_Space; a text never has more code points than bytes, so `bytesize <= 200` implies `size <= 200`; bytes of a multi-byte code point are all above 127; the three refused line breaks (U+0085, U+2028, U+2029) sit in the same character class as the control bytes, so they cost no third scan | 100,000 checks over five sample titles: 86 ms against 1,447 ms for the formula as written (in-process, measured before the line-break clauses were added and not repeated; the baseline is this projection's own literal transcription) |
| `is_tag` as one anchored pattern | every allowed code point is one byte, so size, content and `starts_with("-")` are one scan | not measured |
| `TaskNo` argument: digit scan, then `Integer` | `Int` is exact and unbounded, and so is Ruby's `Integer`; no overflow path exists to guard | not measured |
| JSON reader: punctuation and its white space in one scan; a string with no escape is one scan and one slice | the JSON grammar | 1.04 MB: 112.5 ms before, 55.4 ms after (in-process, same benchmark before and after the change) |
| Command table and help text are frozen at load and `Ractor`-shareable; rules are named by symbol, types are modules | the table is derived from declarations, which do not change at run time | no speed effect measured; it is what lets the table load without loading a rule |
| YJIT is not enabled | none needed: a run is a few milliseconds of mostly cold code | not measured |

Not taken, and why:

| Option | Why not |
|---|---|
| The standard library's `json` for reading | Its C parser is far faster at scale (5.5 ms against 55.4 ms for 1.04 MB) but loading it costs 2.5 ms against 0.27 ms for `strscan`, measured once, and it needs extra checks to meet the contract. From these numbers the hand reader wins below roughly 40 KB (a few hundred tasks) and loses above; a todo board was taken to be small. This is a guess about the workload, not a fact in the spec |
| Decoding `stats` straight from the token stream, no JSON tree | would be a second reader to keep equal to the first; the tree is where 55 of the 79 ms go on the large store |
| Skipping the read for `add` | `add` reads `next_no` and must keep every other task, so the whole store is read and checked |

Read against the parity runner's own median (about 35 ms per run for this backend): the runner starts `exe/todo` through `PATH`, and on this machine `ruby` on `PATH` is a version-manager shim that takes most of that time (32.4 ms through the shim against 7.8 ms direct, `--help`, process). That is the machine's setup, not the projection.

## Could not keep

| Asked | What happened |
|---|---|
| "Ruby 4.0, stdlib only" (contract section 4, toolchain row) | kept at run time. The gate uses three pinned gems, which the same section's rules now allow |
| The projection header on every file | `Gemfile.lock`, `.bundle/config` and `.gitignore` have none: one is generated, the others have no place for it that their readers accept. `.rubocop.yml`, `Gemfile` and both `bin/` scripts carry it as a comment |
| A fused `stats` read that builds nothing it does not need | the generic JSON tree is still built; only `Task`, `Set` and the map are avoided |
| "No mocks" | kept. Rules are called directly; store tests use a temporary directory; `StringIO` stands for standard output in `CLI.run` tests and a real process is used in `test/cli/exe_test.rb` |

## Disagreements

Implemented as the contract says in every case; recorded because the sources pull against each other.

| Where | What |
|---|---|
| `unwritable` for a read | `list` with `TODO_STORE` in a missing directory reports `unwritable` (the lock file cannot be created) though the command writes no store. Correct by the contract; surprising to a user |
| Contract section 4 against itself | the toolchain row says "stdlib only"; the rules below it allow well-known libraries |
| "A rule that writes and succeeds replaces the store atomically" against the skipped identical write | the skip is allowed by the coordinator and by `32-build-layout.md`, but the sentence in section 3 reads as "always replaces". One of the two should say so |

## Line counts

| | Files | Lines | Command (run in this folder) |
|---|---|---|---|
| Product code | 43 | 1,314 | `find exe lib -type f \| sort \| xargs wc -l \| tail -1` |
| Tests | 18 | 1,292 | `find test -type f \| sort \| xargs wc -l \| tail -1` |
| Gate and config | 4 | 120 | `wc -l bin/build bin/check Gemfile .rubocop.yml \| tail -1` |
| Spec projected | 7 | 272 | `cat ../../src/todo/*.slg ../../src/todo/tests/*.slg ../../src/apps/ruby.slg \| wc -l` |

Physical lines, comments and blank lines included. The spec count leaves out the vocabulary card and the contract, which a compiler would also need.

## Limits

- One model, one session, one program. Nothing here shows the projection is repeatable or that another session would make the same choices; the table of open choices is the evidence that it would not without the contract.
- Every measurement is from one machine and one benchmark run, with no variance reported. The json crossover is an estimate from two of those numbers.
- The laws are sampled (250 traces of 40 steps from a fixed seed), not proven. Two of them hold by construction in this design, which the test cannot distinguish from holding by luck.
- The lock is tested within one process and by the parity race of 40 processes on a local file system. Network file systems and crashes between `fsync` and `rename` are not tested.
- Output failures after commit, signals during the lock wait and a full disk during the write are handled only as far as the table of choices says.
