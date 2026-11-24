---
description: Re-project one backend after src/ or docs/contract.md changed, then prove it with its gate and parity.
argument-hint: <rust|bun|ruby|cpp> [what changed]
---

# /project

You stand in for `spec-lang build --target $1`. Change only `out/$1/`.

1. `git diff main -- src docs parity` → the change in meaning, surface and cases. Nothing there → ask what changed.
2. Read the touched `.slg` declarations whole, and the contract sections they reach.
3. Find the files that project them: every file header names its declarations (`grep -rn "<decl>" out/$1`).
4. Change the projection. Keep the layout, the header on each file, the purity of `model`/`render`/`board`.
5. Translate new `test`s, `@cases` and `law`s into native tests with the same names.
6. `out/$1/bin/check` · red → fix, rerun.
7. `bun scripts/parity/run.ts --only $1` then `--race 40` · red → the backend is wrong unless the case contradicts `src/`; then stop and report, never edit the case.
8. `out/$1/NOTES.md`: every new open choice and what you chose; every new optimization with its licence.

Extra context: $ARGUMENTS
