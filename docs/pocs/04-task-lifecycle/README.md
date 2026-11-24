# 04 Task lifecycle

> **Design.** Nothing here runs yet. Workers claim tasks, hold a box while they work, and may die.

**Why this one.** It is the kind of logic where bugs are found in production: a finished holder that still holds, a task nobody owns, work that never ends.

| Must work | Proves |
|---|---|
| a lease with a moving deadline | time inside state |
| at most one live task per box | a law over all orders |
| every task ends | liveness, with fairness stated |
| an old bug put back in is caught | whether laws are worth writing |

**Program:** [program.md](program.md)

## Replay old bugs

Take a real lifecycle you have history for. Keep it outside this repository if it is private. For each past fix in its history:

| Record | |
|---|---|
| the bug in one sentence | |
| caught by a law that was already written | yes, no |
| caught by a law you would have written first | yes, no, be honest |
| needed a fact about the outside | which one |
| not expressible | why |

Choose the fixes **before** writing the laws, from a list made by someone else or by date. Otherwise the laws are written to fit.

## Done when

- The first two laws hold as written; the third holds once the missing law is found and added. Each of these mutations is caught: no expiry, `>` for `>=` in the expiry test, claiming a busy box, finishing after the lease has run out.
- At least 20 past fixes are scored.

## Stop if

Fewer than a quarter of the logic fixes would have been caught by laws written without seeing them.

## Not in scope

A queue service, real workers.
