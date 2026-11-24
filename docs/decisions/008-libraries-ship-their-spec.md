# 008 Libraries ship their spec

**Status:** accepted, 2026-11-24. Nothing is built.

**Decision.** A library or framework that wants spec-lang users publishes, in its own repository and with its own releases, a spec package beside its code:

| File | Holds | Runs |
|---|---|---|
| `contract.slg` | the `pub` types, effects, errors and laws of the calls it offers; what is only `assumed` says so | nowhere: it is read and checked |
| `pretend.slg` | a simulator `handler` that keeps the contract's laws, in memory | in the engine, in every user's tests |
| `vocab.slg` (optional) | kinds and notes, when the library is a framework (`use ultimate;` gives `entity`, `page`) | nowhere: it expands |
| `real.<lang>` and `@needs(...)` | one thin handler per language over the real library, and the version it needs | in the build pool and in production |
| `tests/*.slg` | the contract's tests | in the engine against `pretend`, and in the library's CI against `real` |

A user writes `use payments;`. The author's machine reads the contract (a few hundred tokens), tests against the simulator, and never installs the library. `build` emits code that depends on the real library through that language's package manager.

**Why.** The library's author knows its behaviour better than its first user, already has CI that can run the real thing, and gains users whose agents need no setup to try it. A contract written once by the author replaces one written badly by each user. It is how type definitions settled for JavaScript: they work best shipped by the library, and community copies (`@types/*`) are the fallback.

**The test that keeps it honest.** The same `tests/*.slg` run against `pretend` in the engine and against `real` in the library's CI. A release where they disagree fails. Without that, a simulator drifts from the library and every user's green tests lie.

**Rejected.** *Each user writes the contract* (the default in [35](../idea/35-packages.md) until now): it repeats the work per user and the result is checked against nothing. *Generating contracts from type definitions*: types say nothing about errors, effects, ordering or laws, which are the useful part. *A central contracts registry*: the same objections as in [007](007-packages-over-git.md).

**Cost accepted.** Library authors must learn enough spec-lang to write a contract, and maintain the simulator. Libraries that do not will need a community contract, marked as such, whose evidence is lower: tested by its writer, not by the library's CI.

**Revisit when** library authors decline in practice, or simulators drift from real libraries despite the shared tests.

**Not claimed.** That any library will do this, or how many tokens a contract costs to read; nothing has been published or measured.
