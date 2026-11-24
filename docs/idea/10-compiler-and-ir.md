# Compiler and IR

> **Design.** Nothing here runs yet. The pipeline from source to generated software: IR layers, the claim ledger, the planner, caching, provenance and projections.

**Rule: meaning is kept as long as something can still use it.** A vocabulary kind is not rewritten away on entry. It stays one node, with its expansion beside it, until a backend takes it whole or the planner is forced to expand it.

## Pipeline

```text
source .slg
  -> tree          lossless, one canonical format, one hash per declaration
  -> SIR-D         declaration graph: kinds, notes, laws, tests, reasons
  -> SIR-F         functional core: typed, effects explicit; rules as atomic transitions, tasks as keyed step sequences
  -> SIR-M         machine core: layout fixed, ownership resolved, allocation explicit
  -> target trees  TypeScript, Rust, C, SQL, WGSL, manifests: printed by a backend

  claim ledger     one row per claim, carried beside every layer
  interpreter      runs SIR-F directly; needs no plan, backend or lock file
```

| Layer | Still knows | Passes that live here |
|---|---|---|
| SIR-D | what must hold: `entity Note` with `@access` is one typed node | policy defaults, vocabulary rewrites, pagination and filter pushdown, derived tests |
| SIR-F | algorithms as pure functions over effects; vocabulary kinds as opaque calls with laws; each `task` cut into steps at its rule calls and outside calls, every step with a stable key | fusion, unit erasure, effect commutation, guard ordering |
| SIR-M | bytes, lifetimes, calling convention, interrupt context | layout, inlining, `@alloc` and `@within` checks |
| target trees | one toolchain's idiom, plus provenance ids | printing only |

A pass names the kinds it reads. The pass manager refuses to schedule it after the layer that erases them.

## Retained kinds

Every vocabulary kind has an **expansion** (its meaning, [09](09-vocabularies.md)) and **laws**.

1. The interpreter and the prover use the expansion.
2. Lowering does not. For each node the planner asks whether a backend claims the kind.
3. Claimed: the node passes intact. `entity` plus `@access` becomes one Ultimate entity and policy, not a hand-rolled table and checks.
4. Not claimed: expand one level, ask again.

This is how domain meaning survives long enough to be optimised and how the output lands on the highest-level target available ([11](11-backends-and-parity.md)).

## Claim ledger

Every `@post`, `@guard`, `@within`, `@alloc`, `@access`, label and `law` becomes a ledger row.

```json
{"claim": "notes.Note.access",
 "from": "notes.slg:3",
 "static": "proven",
 "monitored": false,
 "trusts": ["solver:z3@9f1c", "vocab:data@77ab", "backend:ultimate@31e0"],
 "carried_by": "claim(ultimate, data.entity)"}
```

| Rule | Mechanism |
|---|---|
| Nothing is dropped | a pass may discharge a row, restate it in the lower layer's terms, or forward it to a backend claim; a pass that loses a row is rejected |
| Evidence is the triple | static level, monitored or not, trust set, as defined in [04-evidence](04-evidence.md) |
| Backends are trusted by name | a row carried by a backend adds `backend:name@hash` to its trust set; that token has its own evidence, earned by parity |
| Policy decides | `release` fails when any row is below the project's policy; `sketch` fails only on type errors and refutations |

The compiler reports "proven of the meaning, trusting `backend:ultimate@31e0`, which is `sampled`". It never shortens that to "proven".

## Planner

A **plan** maps each SIR-D node to `claim(backend)` or `expand(strategy)`. The algorithm is bottom-up tree covering by dynamic programming over one declaration at a time; constraints across declarations are limited to placement and boundary types. Candidates are ordered by a fixed key:

| Order | Key | Meaning |
|---|---|---|
| 1 | feasible | every ledger row on the node is carried at or above policy |
| 2 | altitude | fewer expansion steps wins: a native claim beats an expansion |
| 3 | static cost | closed-form estimate over declared sizes; measured profiles count only when committed as inputs |
| 4 | strategy name | lexical; there are never ties |

**Strategy catalogue.** Declare-altitude bodies and policy notes resolve through named, versioned strategies with preconditions and a cost formula: `page.keyset` when the order is stable, `page.offset` otherwise; `index.btree`, `index.hash` when no range is asked. The compiler never invents code. If no strategy fits it stops:

```json
{"code": "E-NO-STRATEGY",
 "at": "ledger.slg:41",
 "need": "fn settle(a: Account, b: Account) -> Transfer",
 "tried": [{"strategy": "fold.pairwise", "refused": "post not implied"}],
 "fix": [{"edit": "give a define body", "hole": "?settle"}]}
```

A hole fill is committed source. No model runs in the build.

**Rewriting.** Vocabulary laws drive equality saturation inside one declaration only: relational algebra, units, linear algebra, effect commutation. Fixed rule order, a node cap, an iteration cap, extraction by the key above. Reaching a cap is recorded in the plan as `capped`. Saturation is never used to choose between backends.

**Predictability.**

| Rule | Mechanism |
|---|---|
| Output never drifts | `spec-lang.lock` pins vocabulary hashes, backend hashes, toolchains and the plan per declaration |
| Upgrades are diffs | `spec-lang build --replan` prints "list_notes: page.offset to page.keyset, because keyset now accepts this order"; it lands like source |
| Cone rule | an edit may change plans only in the reverse-dependency cone of the edited declaration; global objectives that break this are forbidden |
| Pins are source | a note argument selects a strategy: `@page(50, by: offset)`, `@index(owner, as: btree)` |

## Incremental and content-addressed

The compiler is one memoised query engine, kept warm as a server ([07-agent-loop](07-agent-loop.md)).

| Thing | Key |
|---|---|
| Declaration | hash of its expanded, name-normalised form; reformatting and renaming a local invalidate nothing |
| Test result | test hash, the hashes of declarations it actually evaluated last run, handler script, seed |
| Proof result | claim hash, solver version, resource limit |
| Emitted file | declaration hashes, vocabulary hashes, backend hash, plan |

A test re-runs only when something it evaluated changed, which is finer than the import graph. Prover timeouts are made deterministic two ways: a stored certificate is replayed where the solver emits one; otherwise "unknown within limit L" is the cached answer and the claim sits at its lower level, in the open. Caches are shareable between machines and agents because the build reads nothing but its declared inputs.

## Provenance

Every IR node carries `origin = (declaration, path inside it, strategy chain)`. Emission writes the target's native debug format (source maps, line directives, DWARF) pointing at `.slg` lines, and a `provenance.json` mapping each emitted span to declaration, strategy and ledger rows. Stack traces, logs, metrics and profiler frames are rewritten through it into declaration names. Nobody needs to read target code to debug ([11](11-backends-and-parity.md)).

## Projections

Humans read projections; only source is edited. A projection is a pure query over IR, ledger and plan, cached like any other artefact, so it cannot be stale and is never an input.

| `spec-lang view --as` | Computed from |
|---|---|
| `expanded` | SIR-D after expansion |
| `guarantees`, and the guarantee diff between two commits | ledger rows |
| `explain` | SIR-D node, its `@why`, the vocabulary's `@explain` text |
| `diagram` | entity graph, `flow` edges, placements, effect rows |
| `policy` | every default in force and where it came from |
| `plan` | `spec-lang.lock` plus provenance: what was generated where and why |
| `ts`, `rust`, `pseudo` | SIR-F through a backend's printer, without planning |

Prose is assembled from templates, so it is deterministic. With dense source, the guarantee diff is the main surface a human approves ([08](08-approval-and-anti-cheating.md)).

## Implementation

The first compiler is written in Rust: it is a long-lived deterministic server with a latency-critical interpreter, it needs cheap hashing and one static binary, and it must later emit machine-altitude code. A small part stays in Rust for good, as the trusted base: core type checker, interpreter, ledger accounting, certificate checker, hashing, lock verifier. Everything else may move into spec-lang ([14-systems](14-systems.md), [20-roadmap](20-roadmap.md)).

## Limits

- **The interpreter is the authority.** A wrong expansion or a wrong core semantics is a wrong meaning that every later stage preserves faithfully.
- **Plan stability fights improvement.** Pins make upgrades a chore; without them output drifts. The cone rule also forbids some real whole-program optimisations.
- **Capped saturation is not monotone.** A small edit can change which rewrites are found before the cap. Recording `capped` exposes this; it does not prevent it.
- **Static cost models are crude.** Without committed profiles the planner will sometimes pick the slower strategy; the remedy is a pin, which is manual.
- **Prover timeouts lower evidence.** Determinism is kept by accepting "unknown", so some true claims will report less than they deserve.
- **Performance of generated code is unmeasured.** Nothing here says how it compares with hand-written target code.
