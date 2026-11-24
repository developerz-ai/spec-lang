# Findings

> **Hand-projected.** Four model sessions projected [`src/`](src/) into four languages, in parallel. What follows comes from their `NOTES.md` files (`out/<backend>/NOTES.md`) and from the parity runs. Nothing here was produced by a spec-lang tool.

## Result

**Done.** All four backends pass their own gates and the 65 parity transcripts, alone and mixed on one store, and a race of 40 concurrent writers across them. Each translated every `@cases`, all ten tests by name, and all six laws as seeded property tests (200 to 400 traces of 40 commands, every law after every step).

## Spec errors

Places where the source was wrong, silent, or weaker than its own `@why`. Each was found by a backend, not by a reviewer.

| # | Found by | What | Status |
|---|---|---|---|
| 1 | Ruby | `is_title` admits U+2028, U+2029 and U+0085, so a title can break a line, against its `@why("fits on one line")` | fixed: the claim refuses them; generated Rust and C++ followed with no human step |
| 2 | Ruby | `is_tag` admits `--` and `--all`, so `list --tag --all` filters by a tag called `--all`, against `@why("typed in a shell without quotes")` | fixed: a tag cannot start with `-` |
| 3 | Rust | `TaskNo` is `Int where self >= 1`, unbounded, but every backend relies on the 2^53 store limit to use a machine integer. The limit is a backend refusal the spec does not state | open: write `type TaskNo = 1..=2^53` and the limit becomes a claim the checker can see |
| 4 | Rust, C++ | the card has no derivation for a parameter with a plain default (`priority: Priority = Normal`): flag or not, repeatable or not | settled in the contract (§2), still missing from [the vocabulary card](docs/cli-vocabulary.md) |
| 5 | C++ | `--priority` given twice: the contract named only `Option` flags | settled: any non-`Set` value flag, case added |
| 6 | C++, Rust, Bun, Ruby | four small surface questions: `-h` as a flag's value, a leading `--`, an empty `TODO_STORE`, a missing value before a repeat | settled in the contract, cases added; three of four backends had already chosen the same way |
| 7 | Rust | the help line's placeholder (`low\|normal\|high` against `<tag>`) cannot be derived from anything written | open: the vocabulary needs a rule for how a sum type and a claim type are shown |
| 8 | Rust | `full` acts like a refusal but exits 3 as a store error; `Contended` exits 1 as if a rule refused, though none ran | open: exit codes per error kind are a question for the `cli` vocabulary |
| 10 | the engine (`spec-lang test`, traces from any lawful world) | `clear_keeps_every_open_task` was false: in a world where a task sits under another number (`47 -> Task(1, ...)`), `clear` removes by `t.no` and leaves the done task. No rule reaches that world, but nothing in the spec forbade it | fixed: law `stored_under_its_number`, which the counterexample proposed |
| 11 | the engine (check, lint) | the program imported non-`pub` names across modules, set another module's state in `@given`, matched replies with `fail(_)`, lacked `@why` on five `pub` rules, and had a law that checked nothing for five of its six rules | fixed in the program; the card now says what a reply and an event are |
| 9 | Ruby, Rust | the contract said both "replaces the store" and "may skip an identical write" | fixed in the contract wording |

## Known differences no case pins

The parity suite cannot send invalid UTF-8 or close standard output, so these differ today and nothing fails:

| Situation | Rust | Bun | Ruby | C++ |
|---|---|---|---|---|
| title argument not valid UTF-8 | `error: argument <n>: not UTF-8` | untested | not a Title | `<title>: not a Title` |
| stdout write fails after commit | dropped, exit 0 | not stated | `SIGPIPE` default; other errors unhandled | `SIGPIPE` default; other errors `error: stdout: unwritable`, exit 1 |
| directory `fsync` after rename | no | no | no | yes |
| JSON nesting when reading the store | schema-directed, never deeper than tags | over 64 unreadable | over 16 unreadable | none stated |

Each row is one open choice that a real compiler would have to make once, in the vocabulary, and all backends would then inherit.

## Surprises

- **The same choices, independently.** Asked nothing, three or four backends picked the same answer to each surface question the contract had missed. Where the source is quiet, a model's default is fairly predictable. That cuts both ways: it is why these gaps stayed hidden until someone wrote them down.
- **The laws held by construction in two designs.** Bun and Ruby noted that in a pure `(world, args) -> world'` design, `refusal_changes_nothing` and `reads_change_nothing` cannot fail. Bun added checks on the store file's bytes so the laws test something. A law that holds by construction in the obvious projection still has value as a monitor, but less as a test.
- **The optimizations came from the effect lines.** Every backend made `list` and `stats` take a shared lock and never write, citing `@uses(tasks.read)`. Each made `stats` count without building tasks, citing the read set. Rust and C++ linked statically, citing the app's `@why("one static binary")`. Each trick in each `NOTES.md` names the spec line that allows it; none needed anything the source did not say.
- **Start-up is the whole cost of a CLI.** Ruby went from 37.6 ms to 7.8 ms for `--help` by adding `--disable-gems`. Bun went from 49.6 ms to 5.4 ms in a crowded directory by turning off config autoloading. Both are facts of the projection: no gems at run time, no config file in the spec.
- **The strict reader is the largest single piece in every backend.** No standard JSON parser in the four languages rejects duplicate keys and integers above 2^53 the way the contract requires, so each wrote its own. The store format was chosen by this POC, not by the language.

## Numbers

Method: one machine (AMD Ryzen 7 5700X3D, Linux 7.0), 2026-11-24, warm cache. Line counts from `bun scripts/stats/loc.ts`. Optimization timings were measured by each projecting session with `hyperfine`, one run, on stores it built; they are in its `NOTES.md` with the command.

| | Rust | Bun | Ruby | C++ |
|---|---|---|---|---|
| projected by | Fable 5.2 | Opus 5.5 | Fable 5.2 | Opus 5.5 |
| agent run time | 17 min | 17 min | 20 min | 13 min |
| product / test lines | 2061 / 1536 | 1122 / 1005 | 1068 / 1041 | 1968 / 1035 |
| open-choice rows in `NOTES.md` | 23 | 24 | 48 | 21 |
| median per command, parity runner | 0.3 ms | 4.0 ms | 34.2 ms | 0.2 ms |

Open-choice rows are table rows under each file's open-choices heading, counted with `grep`; Ruby splits its table by area and lists language-mapping choices the others leave out, so the counts are not comparable one to one. Spec: 179 product lines, 71 test lines. The ratio of projected product lines to spec product lines is 6 to 12; most of the difference is the derived `cli` and `store` plumbing, not the rules.

### The agent's loop: check and test, nothing else

What an agent runs most is the check-and-test loop, so it was timed against each backend's own. Method: `hyperfine`, this machine (AMD Ryzen 7 5700X3D, 16 threads), warm caches, after a one-line change (`touch`) where the target compiles. Memory from `/usr/bin/time`, max resident set of the largest process.

| Loop | What it covers | Time | Peak memory |
|---|---|---|---|
| `spec-lang parse` | form of all 9 files | 1.2 ms | |
| `spec-lang test` | parse, check, lint, 34 checks: 18 case rows, 9 tests, 1 property × 100, 6 laws × 200 traces × 40 steps | 0.80 s, one thread | 4 MB |
| Rust `cargo test` after an edit | incremental debug build, 71 tests | 1.19 s | |
| Bun `bun test` | 203 tests, no build | 0.98 s | |
| Ruby minitest | 94 tests | 4.36 s | |
| C++ rebuild and `ctest` | release build with LTO, 8 test programs | 3.15 s (17.8 s of CPU) | |
| each backend's full gate, nothing changed | format, lint, types, tests, build | Rust 3.9 s, Bun 3.1 s, Ruby 10.1 s, C++ 0.9 s | 80 to 250 MB |

Read it honestly: on a program this small, `spec-lang test` is about as fast as the quickest native test run, not dramatically faster, and most of its time is the 48,000 law steps on one core ([decision 006](../../docs/decisions/006-interpreter-first-in-rust.md) asks for one world per core). What differs is what one run covers and what it needs: one run checks the logic that four languages are generated from, in 4 MB, with no toolchain, while the native loops each check one language and need its compiler, package manager and tools installed. A spec change was then followed by two emitters with no human step (generated Rust and C++ passed the four new parity cases at once), while all four hand-written backends failed them until re-projected.

### The cost of one spec change, by hand and by compiler

The change: `is_title` refuses Unicode line breaks, `is_tag` refuses a leading `-` (two claims, four lines of spec). Method: one agent re-projected the four hand-written backends in order, timing each from first read to green parity with `date +%s`, counting `diff -r` lines against a snapshot; one run, later backends helped by earlier ones.

| | Files | Lines changed | Red gate runs | Agent time | Parity after |
|---|---|---|---|---|---|
| hand-written Rust | 7 | 54 | 1 | 51 s | 68/68 |
| hand-written Bun | 10 | 67 | 3 | 68 s | 68/68 |
| hand-written Ruby | 8 | 60 | 1 | 83 s | 68/68 |
| hand-written C++ | 6 | 55 | 2 | 48 s | 68/68 |
| **hand-written, all four** | **31** | **236** | **7** | **250 s** | |
| generated Rust and C++ | 0 by hand | 0 by hand | 0 | regeneration, not yet timed | 68/68 at once |

Most of the hand cost was tests, not logic: fixtures across all four used values the new claims forbid. And one defect would have shipped silently: the hand-written Bun backend's fast store reader carried **its own copy of the tag pattern** in a regular expression and never called `is_tag`, so changing the claim alone let a store with the tag `-x` read as valid. A person optimising by hand duplicated a claim; a compiler derives the fast path from the claim, so the two cannot drift. That is the case for generated code in one example.

> **Later, 2026-11-24.** The emitters measured above were replaced by general printers ([009](../../docs/decisions/009-one-compiler-any-language.md)). Parity held at 68/68; the speed did not ([README](README.md#generated-by-the-compiler)).

## Verdict

One source of about 250 lines fixed enough meaning that four sessions, working blind to each other in four languages, wrote programs that cannot be told apart on 65 transcripts and share one store under concurrent writes. The rules themselves (`board/`) came out nearly identical in shape across languages. All the disagreement and most of the code was in the derived surface: argument parsing, the store, exit codes. That is the part a vocabulary must state completely, and here a hand-written contract stated it instead. Against the idea: the hand-written contract and vocabulary card (140 non-blank lines) are more than half the size of the spec (250), they carry the whole surface, and four backends still found nine gaps in them or in the source. A compiler would have to derive all of it, so the cost has moved from writing the program to writing the vocabulary. The POC does not show that this trade pays off over many programs; one program cannot.

## Limits

- No spec-lang tool ran. The spec's tests and laws ran only as each backend's translation of them, made by the session that wrote the code.
- Four sessions, two models, one run each. Agreement between them is not evidence about other models or other runs.
- The parity suite was written before the backends, by the coordinator, and grew by five cases during the run when backends found gaps.
- Timings are on small stores and one machine. They compare start-up costs, and should not be read as throughput.
