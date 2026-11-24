# p01-seat-limits

Port of `docs/pocs/01-seat-limits/program.md` to the subset the engine runs. `spec-lang test src/corpus/p01-seat-limits --deny warnings`: 8 checks (2 laws, 6 scenarios), all pass.

| File | Holds |
|---|---|
| `seats.slg` | types, the two states, the four rules, the named start world `one_owner` |
| `laws.slg` | `never_over`, `always_owned` |
| `scenarios.slg` | the five scenarios that start from `one_owner` |

## Cuts

Each row is a form the design writes and the engine does not run. The diagnostic is what `spec-lang test` printed for the form on its own.

| Form in the design | The engine says | The port |
|---|---|---|
| `Id[Org]`, `Id[User]` | `E-UNSUPPORTED` unsupported: the type `Id[Org]` | `type OrgId = 1..=4;`, `type UserId = 1..=4;` |
| `Id(1)` | `E-NAME-UNKNOWN` no `Id` in scope | the literal `1` |
| `@access(owns(by, org: org))` on each rule | `E-UNSUPPORTED` unsupported: the note `@access` on a `rule` | **a model, not the form**: a first guard `@guard(owns(by, org: org) else NotFound)`. It gives what the card promises for a direct call in a test (a denial is `NotFound`, before any guard); it says nothing about exposure, default denial or what a printer must emit |
| `seats.entries` with `it.key`, `it.value` | `E-TYPE-MISMATCH` `Map(..) has no `entries`` , at run time, on the path that reaches it | `seats.keys` and `seats[k]` |
| `@given(orgs[acme] -> Org(limit: 3), ...)` | `E-UNSUPPORTED` unsupported: the `@given` item `orgs[1] -> Org(limit: 3)` | whole places: `@given(orgs -> Map(1 -> Org(limit: 3)), seats -> Map(...))` |

## Other differences

- Three modules, so rules are named `seats.add` from the tests and are `pub`, each with `@why` (`W-LINT-PUB-WHY`); the design's `promote` and `set_limit` had none, so two reasons were written here.
- `always_owned` traces `add | remove | promote`, not `set_limit`: the lint `W-LINT-LAW-INERT` says `set_limit` writes none of `seats`.
- Ids are `1..=4`, not unbounded: with unbounded ids a generated `by` almost never owns the organisation, so every traced call was refused at the first guard and neither law could fail. Measured on mutants: without `set_limit`'s guard `never_over` fails after 1 call, without the `LastOwner` clause `always_owned` fails after 1 call; with `Int where self >= 1` ids both mutants passed both laws.

## Limits

- The access model is a guard written by hand; nothing checks that it equals what `@access` will mean.
- Without `add`'s `Full` guard only the scenario `full` fails; `never_over` did not fail in 200 traces, because limits are drawn from `1..=500` and four users cannot fill them.
- The law parameter `o` is one value per check (the engine's open choice 5), so each law is sampled for one organisation.
- Mutants were run once, by hand, with seed 0.
