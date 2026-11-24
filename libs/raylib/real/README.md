# `raylib` runtimes

Empty: one directory per language goes here, `real/<lang>/` ([decision 009](../../../docs/decisions/009-one-compiler-any-language.md)).

## What a runtime implements

One adapter per declaration of [`contract.slg`](../contract.slg): a call of `foreign c "raylib#Name"` becomes a call of raylib 5.5's `Name` through the language's foreign function interface. At the edge it converts a length from 1/256 of a unit to `float` (`(float)v / 256.0f`), a `Color` to four bytes, a `BoundingBox` to two `Vector3`. It loads raylib the way the platformer's [contract](../../../pocs/platformer/docs/contract.md) section 3 says, and links nothing else.

It reads no manifest: the program's code calls it directly, and an app reaches it through the `game` runtime's window loop.

## Limits

- No adapter exists; the platformer's backends each bind raylib by hand today.
- The library's own tests, run against the real library in its CI as [decision 008](../../../docs/decisions/008-libraries-ship-their-spec.md) asks, do not exist: there is nothing yet to compare a pretend with.
