# 001 Own surface syntax, with a pre-registered fallback

**Status:** accepted, 2026-11-24. Reversible until E0 reports.

**Decision.** spec-lang has its own source language. The semantic core and checker are the product, so the same semantics embedded in TypeScript is the control arm and the fallback.

**Rejected.** *TypeScript front end first*: familiar, but types, effects, units, traces and holes become conventions an agent can ignore, and the host's freedom returns the plumbing. *Natural language*: no single behaviour. *Configuration format*: ends at a wall.

**Revisit when** E0 shows the embedded form within the thresholds in [19](../idea/19-density-and-experiments.md).
