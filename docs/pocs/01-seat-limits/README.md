# 01 Seat limits

> **Design.** Nothing here runs yet. An organisation has a limited number of seats and always at least one owner.

**Why this one.** "At most ten" is a guard over a count: trivial to say, and wrong on most real stores under concurrency.

| Must work | Proves |
|---|---|
| types with ranges, records as map keys | the type checker |
| `@access` with a subject `fn` | denied by default, `NotFound` |
| guards in order with named errors | refusals |
| scenarios that start from other scenarios | test worlds |
| two laws over any order of calls | trace search |

**Program:** [program.md](program.md)

## Done when

- Every test passes and both laws reach `bounded(4)` or better.
- Deleting any one guard makes a law or a test fail, with the calls that break it.
- Changing `<` to `<=` in the seat guard is reported with the boundary case, by name.
- Editing one rule reruns only the tests that evaluated it.

## Measure

Time from edit to verdict, warm and cold; memory; tokens an agent used from an empty file to green, with only the card.

## Stop if

An agent needs more turns here than in TypeScript with an in-memory store and a property-test library, after two revisions of the card.

## Not in scope

Storage, HTTP, a screen.
