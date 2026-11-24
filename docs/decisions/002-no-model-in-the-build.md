# 002 No model in the build

**Status:** accepted, 2026-11-24.

**Decision.** Models author source and fill holes; fills are committed source. The compiler is deterministic and hermetic and never calls a model.

**Rejected.** *Model inside the compiler* (Djinnlang and others): even with proved-unique specifications, builds gain cost, latency and an outside dependency, and outside that proof two builds may differ.

**Consequence.** Where no strategy exists, the build stops with a typed hole instead of inventing code.
