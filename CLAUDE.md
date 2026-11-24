# spec-lang

A programming language for AI authors. **The AI writes logic and tests, the interpreter runs them at once, the compiler generates the software.** Existing languages and frameworks are the compiler's intermediate representation.

**Status `As of 2026-11-24`:** design, plus a first toolchain in `src/`: `spec-lang test` parses, checks, lints and tests the todo POC; emitters for Rust, TypeScript, Ruby and C++ are in progress. Describe nothing beyond what `src/bin/check` and `pocs/*/bin/check` show running.

## Response rules
Execute, no preamble. Read before speculating. Disagree when wrong. Terse. End summary: 2 sentences at most.

## Rules
| Rule | Means |
|---|---|
| The card is the charter | `docs/idea/01-language-card.md` wins; change it first, then the docs that use it |
| Every `slg` fence must parse | grammar in `docs/idea/25-grammar.md`; not source means another fence language |
| Grammar never grows per domain | a domain adds kinds and notes; a new form needs a decision record |
| Admission test | easier for a model to express or to check, and fits an existing form; else library, else rejected with the reason |
| Minimal and sufficient | two possible behaviours is a question, never a guess |
| Honesty | no invented number; a measured one carries method and caveat; every doc ends with Limits |
| Private stays private | no private repo, client, person, host or local path in any file; corpus cases stay generic |
| One path | no shims, no parallel syntaxes; replace completely |
| No worktrees | big work: a team in this checkout, disjoint file sets |

## Map
| Path | Holds |
|---|---|
| `docs/idea/` | the design; index in `docs/idea/README.md` |
| `docs/decisions/` | one record per one-way door |
| `docs/pocs/` | proofs of concept, one folder each, written for an agent with no other context |
| `src/` | the toolchain, Rust: parser, engine, then emitters slg → Rust, TypeScript, Ruby, C++; own `CLAUDE.md` |
| `pocs/` | runnable proofs: spec-lang source, hand-projected backends, their gate; each has its own `CLAUDE.md` |
| `libs/` | libraries a program `use`s: `vocab.slg`, `contract.slg`, `pretend.slg`, `real/<lang>/` runtimes ([009](docs/decisions/009-one-compiler-any-language.md)) |

Markdown only, except `src/`, `pocs/`, `libs/` and the repo tooling in `bin/` and `scripts/`: each code area has its own `bin/check` and CI workflow.

## Commands
```bash
bin/setup                 # toolchains, script deps, every code area
bin/check                 # the gate: docs, src, pocs; `bin/check docs` for prose only. CI runs the same
bin/slg-check <path>      # parse .slg files and slg fences
bun scripts/help.ts       # every script
```
Skill `slg` · commands `/decision`, `/poc`, `/fix-ci` · agent `doc-reviewer`. Run `bin/check` before every commit.

Conventions: the `gold-standards-in-ai` repo.
