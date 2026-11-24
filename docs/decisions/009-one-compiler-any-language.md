# 009 One compiler, any language

**Status:** accepted, 2026-11-24. Built for Rust, TypeScript, Ruby and C++: each printer agrees with the engine on the 41 programs of the judge, and builds the todo app and the platformer. Not built: the optimizer, which the old emitters had in part.

**Decision.** `spec-lang build` compiles any program the engine runs, into any language that has a printer, with three parts and one judge:

| Part | Is | Knows |
|---|---|---|
| **LIR** | one typed, language-neutral lowering of the whole program: modules, records and sums, functions, rules as world transitions (guards, atomicity, rules inside rules), state, tests and laws as data; every name resolved, every type explicit, every library call named | the core language only: no app shape, no vocabulary, no target |
| **printer** | one per language: LIR to source text, plus a small runtime (`Int` that stops instead of wrapping, text by code point, ordered `Map` and `Set`, the world and its rollback, the test runner) | its language's idiom; nothing about any program |
| **library** | `libs/<name>/`, as [008](008-libraries-ship-their-spec.md) says: `vocab.slg` (notes, triggers, effects, app fields), `contract.slg` (`foreign` declarations it implements), `pretend.slg` (what the engine runs in tests), `real/<lang>/` (the runtime and the foreign adapters, per language) | its own domain: `cli` parses arguments and keeps a store, `game` runs a window loop, `raylib` draws |

The compiler emits, for each app, the program's code from LIR and a **manifest**: the app's rules, their parameters and types, triggers and notes, as plain data. A library's runtime reads the manifest; the compiler never contains `cli` or `game`. A `foreign c "raylib#DrawCube"` becomes a call into `libs/raylib/real/<lang>`.

**The judge.** For every program in the corpus, each printer's output runs the program's own checks (`@cases`, tests, properties, laws, same seeds) and must report what `spec-lang test` reports, check by check: pass, fail or blocked. Before any printer, an LIR evaluator must agree with the engine the same way, so a lowering bug is found once and not four times. The corpus: the todo app, the platformer, the five programs in `docs/pocs/`, and one small program per feature of the card the engine runs. The POCs' parity suites stay the outside judge of whole apps.

**Why.** Four emitters that each re-derive types and each embed one app's runtime cost the same again for every language and every vocabulary, so the fifth language, and the first program that is not a todo list, never come. With one lowering, a printer is a few thousand lines of idiom and a runtime; with libraries, a framework (Hono, Solid, Drizzle, Rails) is a library and not a compiler change. The judge makes "any program" a measured claim: the corpus says which programs, the engine says what they mean.

**Rejected.** *Grow the four emitters case by case*: each new program finds the todo shape again in four places. *Emit through one intermediate language* (C or WebAssembly) *and bind the rest to it*: the output must be code a team in that language would keep, which a transpiled C core is not. *Put vocabularies in the compiler with a plug-in API*: a second way to extend the language beside `use`, against [36](../idea/36-extending.md).

**Cost accepted.** The existing emitters are replaced, not extended (one path). The LIR must carry everything a printer needs, so it grows with the card. Each library needs a runtime per language before an app that uses it builds there; a program that uses a library without one gets `E-UNSUPPORTED` naming the library and the language.

**Revisit when** a printer needs program-specific code to pass the judge, or a library cannot be expressed as runtime plus manifest.

**Not claimed.** How many lines a printer takes, or how long a new language takes to add; both get measured when the first ones exist.
