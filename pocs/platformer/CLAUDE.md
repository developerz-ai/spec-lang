# platformer POC

One spec-lang program in `src/`: Skyhop, an original 3D platformer. Its logic is pure spec; its screen is raylib, a C library, declared as `foreign c` in `src/hop/screen.slg`. Projected into four backends twice: `out/` by hand (models standing in for a compiler), `gen/` by `spec-lang build` (never edit; `bin/regen`). One parity suite judges all eight. Models and shaders in `assets/` come from `ts23d` recipes. Never describe `out/` as generated.

## Rules
| Rule | Means |
|---|---|
| `src/` is the source | change meaning there first; then `docs/contract.md` if the surface moves; then a parity case; then every backend |
| Every backend, or none | a change that lands in one backend is a parity failure in seven |
| `gen/` is output | change `src/`, a printer (`../../src/crates/emit-*`) or a library runtime (`../../libs/*/real/`), then `bin/regen` |
| Parity is the judge | `parity/cases/*.txt` fixes bytes and exit codes, and pixel agreement for `shot`; a backend never edits a case to pass |
| A guess is a finding | where `src/` + contract left a choice, write it in that backend's `NOTES.md` and in `findings.md` |
| The C edge is one place | only `hop.screen` and its projection touch raylib; the simulation never does |
| Optimise with a licence | every trick in `out/` names the spec fact that allows it (`NOTES.md` → Optimizations) |
| One owner per folder | agents working in parallel own disjoint paths: `out/<backend>/` each, `assets/` one; `src/`, `docs/`, `parity/` stay with the coordinator |

## Commands
```bash
bin/setup                              # toolchains, raylib into vendor/, deps, build all four
bin/check                              # the gate; CI runs exactly this
../../src/target/release/spec-lang test src   # check, lint and test the spec
bin/regen [--check]                    # rebuild gen/ with spec-lang build, or check it is current
bin/build rust                         # one backend
bin/skyhop cpp play --level 2          # play: arrows or WASD, space to jump
bin/skyhop ruby run < parity/inputs/meadow-win.txt
bun scripts/parity/run.ts --only bun   # parity for one backend
bun scripts/stats/loc.ts               # spec lines against projected lines
bun scripts/help.ts                    # everything else
```

## Where to look
- Language: `../../docs/idea/01-language-card.md`; grammar `../../docs/idea/25-grammar.md`; foreign bodies `../../docs/idea/31-dropping-down.md`
- What a backend must derive: `docs/contract.md`; the `game` vocabulary: `docs/game-vocabulary.md`
- Case format: `parity/README.md`; art: `assets/README.md`
- A backend's choices: `out/<backend>/NOTES.md`; agent `projector`
