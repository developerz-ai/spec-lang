# 02 Ledger

> **Design.** Nothing here runs yet. Accounts, transfers, a history that explains every balance.

**Why this one.** The best-understood logic that still goes wrong: double sends, rounding, a balance and a history that disagree.

| Must work | Proves |
|---|---|
| `@idempotent` with `KeyReused` | delivery policy |
| types that cannot go negative | claims on values |
| conservation as a law | trace search, sums |
| balance equals history, always | a law relating two states |

**Program:** [program.md](program.md)

## Done when

- Both laws hold; sending twice moves once; same key with another amount is refused.
- Writing the two balances from values read before the guard, with the `SameAccount` guard removed, is caught with one call.
- A forced conflict of two transfers gives an outcome equal to some order.

## Stop if

The conservation law cannot be stated without restating the rule.

## Not in scope

Currency exchange, fees, real storage.
