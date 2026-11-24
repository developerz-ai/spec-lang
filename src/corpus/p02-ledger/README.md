# p02-ledger

Port of `docs/pocs/02-ledger/program.md`. `spec-lang test src/corpus/p02-ledger --deny warnings`: 6 checks (2 laws, 3 scenarios, 1 property), all pass.

| File | Holds |
|---|---|
| `ledger.slg` | the whole module, as in the design |

## Cuts

| Form in the design | The engine says | The port |
|---|---|---|
| `Id[Account]`, `Id[Request]` | `E-UNSUPPORTED` unsupported: the type `Id[Account]`; `E-NAME-UNKNOWN` no `Request` in scope | `type AccountId = 1..=4;`, `type RequestId = 1..=9;` |
| `Id(1)`, `Id(9)` | `E-NAME-UNKNOWN` no `Id` in scope | literals; the unknown account in `refusals` is `4` |
| `@idempotent(request)` | `E-UNSUPPORTED` unsupported: the note `@idempotent` on a `rule` | dropped. `request` is still stored in each `Entry`, and a request sent twice moves money twice |
| test `sent_again` (replay, then `-> fail KeyReused`) | rests on `@idempotent` | dropped |
| `@given(accounts[left] -> Account(100), ...)` | `E-UNSUPPORTED` unsupported: the `@given` item `accounts[left] -> Account(100)` | whole places: `accounts -> Map(left -> Account(100), right -> Account(5))` |

## Other differences

- `history_explains` quantifies inside the law (`s.accounts.keys.all(x => ...)`) and has no parameter `x`. As written in the design it fails at once: see Findings.
- `moves` also asserts the history entry, because no law caught a wrong one (Limits).
- `@why` added to the two laws and reworded on `transfer`, whose design reason was about `@idempotent`.

## Findings

- **A law parameter and lawful start worlds disagree.** The design's `law history_explains(t: Trace[transfer], x: Id[Account])` is reported false "after 0 call(s) from a random lawful world", with a start world where the law does not hold for the printed `x`. The start was accepted as lawful for another value of `x`. Minimal program:

```slg
type K = 1..=4;
state a: Map[K, Int];
state b: Map[K, Int];
@guard(k in a and k in b else No)
rule bump(k: K) = { a[k] = a[k] + 1; b[k] = b[k] + 1; }
law same(t: Trace[bump], x: K) =
  t.states.all(s => x in s.a and x in s.b implies s.a[x] == s.b[x]);
```

  It prints `law same is false after 0 call(s) from a random lawful world`, `a -> Map(2 -> 0)`, `b -> Map(2 -> 1)`, `with x = 2`. With the quantifier inside (`s.a.keys.all(x => ...)`) it passes.
- The design asks the engine to "say that such a start exists and show one". It does neither; the evidence line says only how many traces started from lawful worlds.

## Limits

- No rule opens an account, so every trace from a reached world is 40 refusals; only traces from random lawful worlds move money.
- `history_explains` did not catch a mutant that records the wrong `to` in the history (the scenario `moves` does): a random world where an account, its opening balance and a non-empty history agree is rare.
- Without the `Insufficient` guard the claim on `Cents` stops the run with `E-CLAIM`; no law is needed to see it.
- Mutants were run once, by hand, with seed 0.
