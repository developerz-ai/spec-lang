# 05 Elevator

> **Design.** Nothing here runs yet. One car, five floors, one tick per second.

**Why this one.** A simulation with no screen: fixed steps, determinism, and a promise about the future. A small model in an earlier trial wrote an elevator that could starve a floor, which is the bug to catch.

| Must work | Proves |
|---|---|
| `@on(tick(1Hz))` with `clock.advance` | fixed-step time |
| same calls, same run | replay |
| every call answered within a bound | liveness with a number |
| doors never open while moving | a law over steps |

**Program:** [program.md](program.md)

## Done when

- The laws hold over random and adversarial calls.
- Replacing the direction rule with "always go up first" is caught, with the calls that starve a floor.
- The state hash after 10,000 ticks is the same on Linux, macOS and Windows.

## Measure

Ticks per second in the engine; memory per world; worlds in parallel per core.

## Stop if

Ticks per second are so low that a bounded check of 100 ticks is not interactive, and bytecode would not change that.

## Not in scope

Weight, several cars, a screen.
