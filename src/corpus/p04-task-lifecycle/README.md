# p04-task-lifecycle

Port of `docs/pocs/04-task-lifecycle/program.md`. `spec-lang test src/corpus/p04-task-lifecycle --deny warnings`: 10 checks (4 laws, 6 scenarios), all pass.

| File | Holds |
|---|---|
| `tasks.slg` | the whole module |

## The clock is a model

The design reads `now()` and is driven by `clock.advance`. The engine has neither. The port keeps time as plain state, `state time: Seconds = 0`, moved only by the rule `pass(d)`. This is ordinary spec-lang the engine runs; it is **not** the form: no handler, no `clock.read` effect, no timer. `expire` fires nowhere by itself: a test that advanced the clock in the design calls `pass(n); expire();`, and a trace calls the two in any order.

## Cuts

| Form in the design | The engine says | The port |
|---|---|---|
| `Id[Task]`, `Id[Box]`, `Id(1)` | `E-UNSUPPORTED` unsupported: the type `Id[Task]`; `E-NAME-UNKNOWN` no `Box` in scope; no `Id` in scope | `type TaskId = 1..=4;`, `type BoxId = 1..=3;`, literals |
| `until: Time`, `const LEASE: Time` | `E-NAME-UNKNOWN` no `Time` in scope | `Seconds = Int where self >= 0` |
| `5min`, `1s` | `E-UNSUPPORTED` unsupported: the number `5min`: only whole numbers without a unit | `300`, `1` |
| `now()` | `E-NAME-UNKNOWN` no `now` in scope | the state `time` (model) |
| `@uses(tasks.write, clock.read)` | accepted; `W-LINT-EFFECT-UNUSED`: `@uses` lists `clock.read`, but the body never has that effect | no `@uses`: the rules are private, their effects inferred |
| `@on(every(1s))` on `expire` | `E-UNSUPPORTED` unsupported: the trigger `every(1s)` | dropped; `expire` is called directly |
| `x.is(Running(box, until)) and now() < until` (a pattern that binds) | `E-UNSUPPORTED` unsupported: a pattern in `.is(...)` that binds names | `fn held`, `fn overdue`: a `match` that binds. `.is(Running(box, _))`, which only compares, is kept |
| `tasks.entries`, `e.key`, `e.value` | `E-TYPE-MISMATCH` `Map(..) has no `entries``, at run time | `for k in tasks.keys` and `tasks[k]` |
| `Trace[... \| crash]` | `E-NAME-UNKNOWN` no `crash` in scope | `crash` dropped from every trace |
| `t.within(LEASE + 1s, p)` in `a_dead_worker_lets_go` | `E-UNSUPPORTED` unsupported: `t.within`: liveness over traces | the law is dropped. Two safety laws stand where it was: `lease_is_bounded` (the state law the design says is missing: `until` is never more than `LEASE` ahead) and `expire_frees_the_overdue` (after an `expire` step no lease is overdue) |
| `@given(clock.at(0s), tasks[t1] -> Task(), ...)` | `E-UNSUPPORTED` unsupported: the `@given` item `clock.at(0s)` (handlers are not implemented); the same for `tasks[t1] -> Task()` | `@given(tasks -> Map(t1 -> Task(), t2 -> Task()))`; `time` starts at 0 |
| `clock.advance(5min);` | `E-NAME-UNKNOWN` no `clock` in scope | `pass(300); expire();` |

## Other differences

- `one_live_task_per_box` quantifies over boxes inside (`(1..=3).all(b => ...)`), not with a parameter `b` (the finding in `../p02-ledger/README.md`).
- `one_live_task_per_box` and `ended_holds_nothing` do not trace `pass`: `W-LINT-LAW-INERT` (it writes none of `tasks`).
- Tests added: `a_late_finish_is_refused_before_expire_runs` (the lease is over, `expire` has not run), `an_unknown_task_is_not_queued`, and an exact stored value in `two_waiting`.

## Limits

- Nothing about liveness is checked: a dead worker letting go within a bound is exactly what was cut.
- Time in whole seconds moved by a rule is not a clock: no timer order, no crash, no "the same `now()` for a whole rule" to check.
- No rule creates a task, so traces from a reached world are all refusals; the laws are sampled from random lawful worlds only.
- Mutants caught by a law: `finish` without its guard, a lease one second too long. Caught only by scenarios: `claim` without `Busy`, `held` ignoring the box and the time, `overdue` with `>`, `expire` never failing a task. `claim` without `Spent` stops with `E-CLAIM` on `tries`. Run once, by hand, with seed 0.
