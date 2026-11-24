---
description: Start a runnable proof of concept under pocs/<name>/ with the same shape as pocs/todo-app: spec, contract, parity suite, gate, CI.
argument-hint: <name> <what it should prove>
---

# /poc

A POC exists to find where the language is wrong. Done means its `bin/check` is green and `findings.md` says what was learned, including what argues against the idea.

1. Read `docs/pocs/README.md` (the method) and `pocs/todo-app/README.md`, `CLAUDE.md`, `findings.md` (the shape).
2. Write the program first: `pocs/$1/src/**.slg`. `bin/slg-check pocs/$1/src` green before anything else.
3. Write what a compiler would derive in `pocs/$1/docs/contract.md`: every byte a user or another program sees. Two behaviours left → decide and say so.
4. Write the judge before any backend: language-neutral cases, a runner under `pocs/$1/scripts/`.
5. Backends: one `projector` agent per backend (see `pocs/todo-app/.claude/agents/projector.md`), disjoint folders `out/<backend>/`, in parallel, in this checkout.
6. Gate: `pocs/$1/bin/{setup,build,check}`; CI `.github/workflows/$1.yml` running exactly `bin/check`, actions pinned to SHAs. `bin/check pocs` picks it up.
7. `findings.md` (Result, Spec errors, Surprises, Numbers with method, Verdict, Limits) and a row in `docs/pocs/README.md`.

What to prove: $ARGUMENTS
