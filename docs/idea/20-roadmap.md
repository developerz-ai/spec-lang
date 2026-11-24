# Roadmap

> **Design.** Nothing here runs yet. Order of work, with a gate that can stop each step.

Cheapest disproof first. A milestone is a demo plus a fixture that must be green.

| # | Milestone | Delivers | Gate | Stop if |
|---|---|---|---|---|
| **E0** | Is it worth a language? | the semantics as a tiny TypeScript library with an interpreter, a hand-translated spec-lang corpus, blind-author trials on three model families | author, edit and predict tasks, spec-lang against the embedded form | spec-lang shows no plausible advantage: ship the embedded form |
| **M1** | Logic loop, one Rust binary ([006](../decisions/006-interpreter-first-in-rust.md)) | parser, formatter, types, interpreter, holes, `view edit test`, hash-scoped tests, simulated clock, store and network | the webhooks, seats and ledger modules run; edits re-run only affected tests | models need more turns to green than in TypeScript after two card revisions |
| **M2** | First vocabularies | `entity`, `query`, `flow`, effect policies, typed failures, retention and schedules | corpus cases 1, 4 and 6 re-expressed; their existing tests used as black-box oracles | under half of production tokens disappear with behaviour kept |
| **M3** | Ultimate backend | generated files in a real app, parity harness, `build` | the same tests agree on interpreter and app; Ultimate's own gate is green | one hand-edited generated file is needed to ship |
| **M4** | Trust | claims, evidence ledger, bounded checks, SMT, guarantee diff, `prove land` | seeded-bug suite: zero false "proven"; humans catch seeded weakenings from projections | reviewers do worse than with a normal PR diff |
| **M5** | Second target | one different backend (Rust service or native mobile), boundary types, fleet of agents on one checkout | one module, two targets, one test suite | second backend costs more than the first |
| **M6** | Machine altitude | ownership, layout, interrupts, `foreign` | ring buffer and page allocator on hardware or an emulator | no safety gain over Rust on the same tasks |
| **M7** | Games | anygame backend, if anygame's schema, ECS and headless runner exist by then | one deterministic system, state hash equal to the interpreter | anygame is not ready: target another engine or wait |
| **M8** | Systems and self-hosting | key-value store with crash testing, small kernel, parser and checker in spec-lang | crash enumeration finds seeded bugs | stays research; never blocks M1 to M5 |

The compiler's first implementation is Rust, with a small permanent trusted kernel ([10-compiler-and-ir](10-compiler-and-ir.md)). Distribution is npm (`bunx spec-lang`) with a native binary per platform, because the first users live in Bun apps.

## Proofs of concept

Each has a folder in [docs/pocs](../pocs/README.md).

The engine grows with small programs, and each program is chosen to break something. Engine and backends are fixed as they go.

| POC | Drives | Answers |
|---|---|---|
| seat limits, ledger | parser, types, rules, tests | is it good to write; is feedback fast |
| invites, webhooks | flows, events, tasks, simulated time | do lifecycles and retries hold up |
| a task lifecycle, with its past bugs replayed | trace search, laws | would laws have caught real bugs |
| webhooks inside an existing app | the first backend, parity | does the ratio hold; do the tests agree |
| a small headless simulation | determinism, fixed step | is it any use beyond the web |
| one module on a second backend | boundary types, parity | is it platform independent |

The first three need no backend. Three rules:

1. A bug a POC finds becomes a test of the engine.
2. When engine and [card](01-language-card.md) disagree, the card is settled first. Otherwise the code becomes the specification.
3. At least one POC is built twice, once on the plain framework, by the same agent under the same effort. That is the measurement; the rest are experience.

This runs beside E0 and does not replace it: a POC shows that something can be built, and E0 whether it was worth a language.

## Not scheduled

A general UI language (presentation code was not shown to be mechanical), an editor, a package registry, full dependent types, a hosted service.

## Limits

No dates: no part has been built, so none could be honest. Thresholds above are proposals until frozen before each sealed run ([19-density-and-experiments](19-density-and-experiments.md)).
