# p05-elevator

Port of `docs/pocs/05-elevator/program.md`. `spec-lang test src/corpus/p05-elevator --deny warnings`: 13 checks (8 `@cases` rows, 2 laws, 2 scenarios, 1 property), all pass.

| File | Holds |
|---|---|
| `lift.slg` | the whole module |

## Cuts

| Form in the design | The engine says | The port |
|---|---|---|
| `@on(tick(1Hz))` on `move` | `E-UNSUPPORTED` unsupported: the trigger `tick(1Hz)` (the module does not `use game`) | dropped; `move` is called directly, one call per tick |
| `@given(clock.at(0s))` | `E-UNSUPPORTED` unsupported: the `@given` item `clock.at(0s)` (handlers are not implemented) | dropped |
| `clock.advance(2s);`, `clock.advance(20s);`, `clock.advance(0s);` | `E-NAME-UNKNOWN` no `clock` in scope; `2s` alone: `E-UNSUPPORTED` unsupported: the number `2s`: only whole numbers without a unit | that many direct calls of `move()` |
| `t.within(40s, not (f in it.calls))` in `every_call_is_answered` | `E-UNSUPPORTED` unsupported: the number `40s`: only whole numbers without a unit; without the unit: unsupported: `t.within`: liveness over traces | the law is dropped. In its place a property, `two_calls_are_answered_in_forty_ticks(f, g)`: from the start world, two calls and 40 ticks leave no call. It covers one start world and no call arriving on the way |

`@uses(calls.write)` and `@uses(lift.write, calls.write)` are left out: the rules are private and the engine infers their effects.

## Other differences

- `@cases` on `step`: eight rows, among them a turn at each end of the shaft.
- A law added, `only_an_open_door_answers`: a call leaves the set only at the floor where the car stands open.
- `doors_shut_while_moving` traces `move` alone: `W-LINT-LAW-INERT` says `call` writes none of `lift`. See Findings.

## Findings

- `W-LINT-LAW-INERT` judges by the states a rule writes. `call` writes `calls`, which `move` reads to decide where `lift` goes, so a trace of `call | move` reaches worlds a trace of `move` reaches only through its start. The lint cannot be met without narrowing the trace.
- The design asks whether the checker sees that `l.at + 1` stays a `Floor`. The engine monitors the claim at run time and never broke it in 400 traces and 100 property runs; a `step` that never turns stops with `E-CLAIM` (`field `at` is not a Floor: 6 is not in 1..=5`). Nothing is proven.

## Limits

- The design's question, the true bound on a wait, is not answered: no liveness, no search for a bound.
- A tick as a direct call has no rate and no clock.
- Mutants caught: doors never closing (a `@cases` row and `doors_shut_while_moving`), `remove` without the open door (`only_an_open_door_answers`), never turning and always going up (`E-CLAIM`). `ahead` with `>=` is equivalent and survives. Run once, by hand, with seed 0.
