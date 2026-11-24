# Proofs of concept

> **Design.** Nothing here runs yet. Small programs that drive the spec engine and the first backends, written so an agent with no other context can start.

| # | POC | Drives | Needs a backend |
|---|---|---|---|
| 01 | [seat limits](01-seat-limits/README.md) | types, rules, guards, access, laws | no |
| 02 | [ledger](02-ledger/README.md) | money, idempotency, conservation | no |
| 03 | [invites and webhooks](03-invites-and-webhooks/README.md) | flows, deadlines, events, tasks, retries | no |
| 04 | [task lifecycle](04-task-lifecycle/README.md) | leases, races, liveness, replaying old bugs | no |
| 05 | [elevator](05-elevator/README.md) | fixed ticks, determinism, liveness | no |
| 06 | [webhooks on Ultimate](06-webhooks-on-ultimate/README.md) | first backend, parity, the honest measurement | yes |
| 07 | [a second backend](07-second-backend/README.md) | platform independence | yes |
| 08 | [todo app, four backends](../../pocs/todo-app/README.md) | one source to Rust, TypeScript, Ruby and C++, by hand and by `spec-lang build`, one parity suite | yes, four |
| 09 | [platformer, four backends and a C library](../../pocs/platformer/README.md) | a fixed-tick 3D game: pure rules, laws over play, `foreign c` bindings to raylib in four languages, pixel parity | yes, four |

Do them in order. Stop after any of them if its stop rule is met.

## For the agent that picks one up

1. Read the [language card](../idea/01-language-card.md), then the POC's `README.md` and `program.md`. For engine work add [25](../idea/25-grammar.md), [28](../idea/28-standard-library.md), [30](../idea/30-type-rules.md), [06](../idea/06-logic-and-tests.md) and [decision 006](../decisions/006-interpreter-first-in-rust.md).
2. Make `program.md` parse, check and run. Change the engine, not the program, unless the program is wrong.
3. When engine and card disagree, stop and settle the card. Change the card and the docs that use it in the same commit.
4. A bug found becomes an engine test before it is fixed.
5. Write `findings.md` in the POC folder.

## What to write in findings

| Section | Holds |
|---|---|
| Result | done, partly, stopped; which tests and laws pass |
| Spec errors | what in the docs was wrong or missing, and the change made |
| Surprises | where the language fought you; what you wanted to write |
| Numbers | method first: command, machine, model, runs. Then tokens, turns to green, time, memory |
| Verdict | one paragraph, including what argues against the idea |

No number without its method. A result against the language is as useful as one for it.

## Status of these programs

They parse with the [grammar](../idea/25-grammar.md). The logic of 01, 04 and 05 was checked as Python transliterations: 3,000 to 4,000 random runs each, with the laws as assertions. That checks the idea and not this text. Nothing has been type-checked or run in spec-lang.

## Limits

Seven small programs show what is possible in small programs. Size, teams and years are not tested here.
