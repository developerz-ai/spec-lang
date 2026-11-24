# 07 A second backend

> **Design.** Nothing here runs yet. One module, two targets, one suite.

**Why this one.** "Platform independent" is worth nothing until it has happened twice.

**Program:** the ledger from [02](../02-ledger/program.md), unchanged.

| Candidate | Chosen if |
|---|---|
| Rust service with an embedded store | the aim is meaning on a very different target |
| Ruby on Rails | the aim is daily use in an existing app |
| SQL functions | the aim is rules inside the database |

## Done when

- The module's source is byte-identical for both.
- The same tests, the race tests included, agree in the engine and on both.
- Anything one backend cannot keep is refused by name.
- The cost of the second backend is recorded next to that of the first.

## Stop if

It costs more than the first. Then the shared part is too small, and breadth is not affordable.
