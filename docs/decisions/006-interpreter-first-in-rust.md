# 006 Interpreter first, in Rust

**Status:** accepted, 2026-11-24. Nothing is built.

**Decision.** The first program built is the spec engine: the interpreter with its checker and test runner, in Rust, as one binary per system. No backend until it is useful alone.

**Why Rust.** No garbage collector, so memory is predictable and low; safe threads; one static binary for Linux, macOS and Windows, and a WebAssembly build for a browser tab. It is also the compiler's language, so the front end is written once.

**Rejected.** *TypeScript on Bun*: quickest to start and right for the E0 experiment, but a collector and one thread per isolate work against many worlds at once. *C++*: as fast, and data races are ours to find. *Go*: simple, with a collector. *A JVM language*: start-up and memory.

**What makes it fast and small** (Rust alone does not):

| Need | Choice |
|---|---|
| fork a world at no cost | immutable values with shared structure; a fork copies a root |
| every core | one world per worker thread, nothing shared but immutable code; work stealing over tests, seeds and start states |
| no contention | counts of shared values stay inside a thread; results merge at the end in a fixed order |
| low memory | an arena per world, freed at once; values packed, not boxed one by one |
| quick start | parsed and checked declarations cached by hash |
| speed per step | compact bytecode; a compiler for hot code only if profiles ask for it |
| same answer everywhere | no hash order, no clock, no host floats beyond the specified set |
| a hostile program | step and memory budget per world; no file or network access in the crate |

**Revisit when** the E0 experiment says the language is not needed, or profiles show the interpreter is not where time goes.

**Not claimed.** No speed or memory figure. Targets are set once a first version can be measured: modules per second, memory per world, cold start.
