---
description: Pull the failing CI job, reproduce it with the scoped local gate, fix the cause, rerun.
---

# /fix-ci

1. `gh run list --limit 5` → the failing run; `gh run view <id> --log-failed` → the first real error, not the last line.
2. Map the job to its local gate: `docs` → `bin/check docs`; `compiler` → `src/bin/check`; `todo-app` → `pocs/todo-app/bin/check`.
3. Run that gate locally. Green locally but red in CI → the difference is the bug (a tool version, a missing setup step, a path); fix the setup, not the test.
4. Fix the cause. Never loosen a check, skip a case, or add `continue-on-error` to go green.
5. Rerun the scoped gate, then `bin/check`.
