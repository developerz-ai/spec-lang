# todo-app POC

One spec-lang program in `src/`, built into four backends twice: `out/` by hand (models standing in for the compiler, now the reference), `gen/` by `spec-lang build` (never edit; `bin/regen`). One parity suite judges all eight. Never describe `out/` as generated.

## Rules
| Rule | Means |
|---|---|
| `src/` is the source | change meaning there first; then `docs/contract.md` if the surface moves; then a parity case; then every backend |
| Every backend, or none | a change that lands in one backend is a parity failure in seven |
| `gen/` is output | change `src/` or an emitter in `../../src/crates/emit-*`, then `bin/regen`; the gate fails on a stale `gen/` |
| Parity is the judge | `parity/cases/*.txt` fixes bytes, exit codes and the store; a backend never edits a case to pass |
| A guess is a finding | where `src/` + contract left a choice, write it in that backend's `NOTES.md` and in `findings.md` |
| Optimise with a licence | every trick in `out/` names the spec fact that allows it (`NOTES.md` → Optimizations) |
| One owner per folder | agents working in parallel own disjoint paths: `out/<backend>/` each; `src/`, `docs/`, `parity/` stay with the coordinator |

## Commands
```bash
bin/setup                              # toolchains, deps, build all four
bin/check                              # the gate; CI runs exactly this
bin/regen [--check]                    # rebuild gen/ with spec-lang build, or check it is current
../../src/target/release/spec-lang test src   # check, lint and test the spec
bin/build rust                         # one backend
bin/todo ruby add "buy milk" --tags home
bun scripts/parity/run.ts --only cpp   # parity for one backend; --mix, --race 40
bun scripts/stats/loc.ts               # spec lines against projected lines
bun scripts/help.ts                    # everything else
```

## Where to look
- Language: `../../docs/idea/01-language-card.md`; grammar `../../docs/idea/25-grammar.md`
- What a backend must derive: `docs/contract.md`; the `cli` vocabulary: `docs/cli-vocabulary.md`
- Case format: `parity/README.md`
- A backend's choices and tricks: `out/<backend>/NOTES.md`
- Workflows: `/project <backend>`, `/parity`, `/new-backend <lang>`; agents `projector`, `parity-reviewer`; skill `spec-lang-authoring`
