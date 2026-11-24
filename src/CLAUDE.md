# src/: the spec-lang toolchain

Rust workspace: parser, IR, check, lint, interpreter, test engine, CLI. Read `README.md` here first; the language is `../docs/idea/01-language-card.md`, the grammar `../docs/idea/25-grammar.md`.

## Rules
| Rule | Means |
|---|---|
| The grammar is the reference | the parser accepts exactly what the Lark grammar accepts; a parser change runs `bin/lark-diff` on several seeds |
| Never a silent wrong answer | a form the engine does not implement is `E-UNSUPPORTED` at `file:line`; never skip, never guess |
| Two behaviours is a question | when docs or the program allow two meanings, pick one, then list it in `README.md` "Open choices" with `file:line`; never edit `../docs` or `../pocs` from here |
| Every diagnostic is a `Diag` | stable code registered in `crates/diag/codes.md`, span, declaration path, `at`, expected/found, edits; the registry test fails on an unregistered code |
| Honest evidence | examples print `example`, sampling prints `sampled(n)`; nothing prints `proven` |
| Determinism | no hash order (BTreeMap/BTreeSet), no clock, own RNG; a seed reproduces a run; output never depends on `--jobs` (runs merge by number, `tests/jobs.rs`) |
| One concern per module | small files; library code does not `unwrap`, `expect` or `panic` (workspace lints deny them) |
| A failing program is a finding | if the todo POC fails a check, suspect the engine first; if the program is wrong, report it, do not edit it |
| Mutants stay listed | `crates/cli/tests/mutations.rs`: a mutant's fate changing fails the test; update its list and the README table together |
| Owned elsewhere | `crates/emit-*` and `crates/view` belong to other agents; the IR is their API: change it additively and say so |

## Commands
```bash
heavy bin/check                                   # the gate: fmt, clippy -D warnings, all tests, parse docs, test the POC
cargo test -p spec-lang-syntax                    # one crate
cargo test -p spec-lang-cli --test mutations -- --nocapture   # the mutation table
cargo build --release -p spec-lang-cli
target/release/spec-lang parse ../docs ../pocs/todo-app/src ../README.md
target/release/spec-lang test ../pocs/todo-app/src [--json] [--seed N] [--deny warnings]
target/release/spec-lang view ../pocs/todo-app/src board.done [--json]
target/release/spec-lang build ../pocs/todo-app/src --out <dir> [--app <name>]
bin/lark-diff <seed> <count>                      # parser against Lark on mutated inputs (needs uv)
```

## Where things are
| Want to change | File |
|---|---|
| a token or keyword rule | `crates/syntax/src/lexer.rs`, `parser/mod.rs` (contextual keywords) |
| a precedence level | `crates/syntax/src/parser/expr.rs` (one function per grammar level) |
| what a name means | `crates/ir/src/lower/expr.rs`, `scope.rs` |
| which notes exist, where | `crates/ir/src/lower/notes.rs` |
| a library method | `crates/engine/src/stdlib/` |
| rule atomicity, claims | `crates/engine/src/interp.rs`, `conform.rs` |
| generation, traces, laws | `crates/testing/src/generate.rs`, `trace.rs`, `law.rs` |
| a lint | `crates/check/src/lint.rs` |
| a diagnostic code | `crates/diag/codes.md` |
