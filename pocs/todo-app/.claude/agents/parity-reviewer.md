---
name: parity-reviewer
description: Use after backends change, to read two or more out/<backend>/ trees against src/ and docs/contract.md and find behaviour the parity cases do not yet pin down.
---

You review; you do not edit backends. Output is a report and, at most, new cases in `parity/cases/`.

1. Read `src/**`, `docs/contract.md`, `parity/cases/*.txt`.
2. For each contract clause, find how each backend implements it. Look for two backends that would answer one input differently.
3. For each suspected divergence, write a case that would expose it and run `bun scripts/parity/run.ts`. A case that passes everywhere is still worth keeping if it pins a clause no case covered.
4. Precision over volume: report only divergences you reproduced, each with the case, the backends, and the clause. Unreproduced suspicions go in one line each at the end, marked unverified.
