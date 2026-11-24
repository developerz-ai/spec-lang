---
description: Add a fifth backend (Go, Kotlin, Zig, ...) by projecting src/ against docs/contract.md, then wire it into parity, the gate and CI.
argument-hint: <name> <language and toolchain>
---

# /new-backend

1. Read `docs/contract.md` section 4 and one finished backend's `NOTES.md` for the shape, not the code.
2. Brief one `projector` agent with: name `$1`, toolchain from $ARGUMENTS, folder `out/$1/`, executable path.
3. While it works: add `$1` to `parity/backends.json`, to the backend loops in `bin/build` and `bin/check`, a column to contract section 4, its toolchain to `bin/setup` and `.github/workflows/todo-app.yml`.
4. Done when `bin/check` is green with five backends and `--mix` rotates through all five.
5. Record its cost next to the others in `findings.md` (time, lines, open choices it found that four backends had not).
