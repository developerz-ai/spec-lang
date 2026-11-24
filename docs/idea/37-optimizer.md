# Optimizer

> **Design.** The four hand projections in [the todo POC](../../pocs/todo-app/README.md) apply some of these. Where a table below says "generated", it means the first emitters, since replaced by general printers that apply none ([009](../decisions/009-one-compiler-any-language.md)); no planner exists. How the compiler produces code better than a careful person writes, by borrowing what gcc, LLVM, the JVM and other JITs, databases and functional compilers already know, and why it can go further than they do.

**Rule: an optimization is a named strategy whose precondition is a fact the source states.** It is never a guess about intent. gcc must prove that two pointers do not alias before it reorders loads; spec-lang is told, by `@uses`, read sets, claims and laws, what a rule touches and what must hold. So the catalogue of 50 years of compilers applies, with better facts. The [strategy catalogue](10-compiler-and-ir.md) holds each one: preconditions, cost formula, version.

## How generated code is made

```text
SIR-F (typed, effects explicit)
  -> analyses        read and write sets, purity, ranges from claims, uniqueness, cardinality from types
  -> passes          rewrite SIR-F, each step checked: same meaning, by construction or by the laws
  -> plan            pick a strategy per node by cost, using committed profiles when present
  -> target AST      syn, swc, or an own typed AST; never text glued together
  -> printer, formatter, the target's own parser as a check
  -> the target's compiler does what it does best: registers, inlining, vectorising
```

Four rules keep it honest.

| Rule | Means |
|---|---|
| Correct first, then fast | a strategy is turned on only after the plain translation passes every test, law and parity case, and it must pass them again; a faster wrong program is a refused build, never a trade-off |
| Leave local work to the target | rustc, clang and JSC already allocate registers, unroll and vectorise. The planner does what they cannot: decisions that need the meaning of a whole rule or program |
| Every pass is checked | a pass either preserves meaning by construction, or the program's own tests and laws run on the result before it is accepted. Translation validation, with the oracle already written |
| Output stays readable | names from declarations, one file per concern, an anchor to the `.slg` line before each guard and statement, and `provenance.json` naming the strategy and the fact behind it |

## The catalogue

| Technique | Borrowed from | Licensed here by | In the todo POC |
|---|---|---|---|
| **Common subexpression elimination**: one lookup serves both guards and the write | gcc, LLVM GVN | `tasks[no]` is pure within a rule | generated Rust, hand-written |
| **Range analysis, check elimination**: `no + 1 >= 1` needs no runtime check | LLVM range analysis | the claim `TaskNo = Int where self >= 1` | generated Rust (`from_proven`) |
| **Integer narrowing**: a machine word instead of a big integer | LLVM, JIT type specialisation | a claim or a refusal bounds the value; the bound must be stated, never assumed | hand-written, all four (2^53 store limit) |
| **Read-only specialisation**: a shared lock, no writer reached | effect systems, database read-only transactions | `@uses(tasks.read)` and a body that assigns nothing | all, hand and generated |
| **Projection pushdown, fused read**: count statuses while parsing, never build the world | SQL planners, stream fusion | the rule's read set is one field | hand-written Rust, C++, Bun; **missing in generated code: 2.3× to 4.4× slower on 100,000 tasks** |
| **Deforestation**: `filter`, `map`, `size` with no intermediate list | Haskell stream fusion, Rust iterators | pure collection operations, `fold` order fixed by the card | partly, through target iterators |
| **Copy elision, in-place commit**: mutate a draft, publish once | escape analysis, persistent data structures | a rule is atomic and the process commits once | **missing in generated TypeScript: copies the map per command** |
| **Dead store elimination**: an identical store is not rewritten | gcc DSE, build systems | equal bytes mean an equal world ([32](32-build-layout.md)) | hand-written, all four |
| **Unstable sort when keys are unique** | standard libraries | `rank` ends in `no`, unique by the map's key | not yet |
| **Constant folding, partial evaluation**: help text, parameter tables, `@cases` | gcc, Futamura | pure with constant inputs is computed in the build ([card](01-language-card.md)) | all: static tables |
| **Speculation with a fallback**: a fast parser for the exact bytes this program writes, the strict one otherwise | JIT speculation and deoptimisation | the store's canonical form is fixed by the backend | hand-written Bun |
| **Intrinsics**: `size` on ASCII text is its byte length; `bytes.all(...)` becomes a SIMD scan | gcc builtins, LLVM intrinsics | core library meanings are fixed ([28](28-standard-library.md)) | ASCII fast paths, hand-written |
| **Incremental maintenance**: keep `open` as a counter instead of recounting | materialised views, database indexes | the module owns its state, so every writer is known | not yet; a strategy for when profiles say `stats` is hot |
| **Profile-guided choice** | gcc PGO, JIT tiering | the engine already runs thousands of traces; their counts are a workload model, and a committed production profile overrides it | not yet |
| **Start-up as a budget**: static linking, bytecode, no gems, lazy loading | AOT compilers, JIT warm-up work | `@why("one static binary")`; nothing in the spec needs a config file | hand-written, all four |
| **Closed-world compilation**: drop what no rule reaches, devirtualise every call, initialise tables at build time | GraalVM native-image, Project Leyden | an `app` lists every rule it exposes; there is no reflection and no class loading | partly: static tables, dead code left to the target |
| **Escape analysis, scalar replacement**: a value that never leaves a rule lives in registers, not on the heap | HotSpot C2, Graal | values are immutable and a rule's results leave only through its reply and its writes | left to the target compiler |
| **Inline caches, speculation on the common case**: a fast path for the variant seen most, a guarded fall-back for the rest | HotSpot, V8, JavaScriptCore | sum types are closed, so the planner knows every case and can order them by profile | not yet |
| **Deoptimisation as a contract**: a specialised path is valid only while its assumption holds, and falls back when it fails | HotSpot deopt, Truffle | a strategy's precondition is a stated fact; a pinned strategy whose fact disappears is a build error, not a slow path | by design in the strategy catalogue ([10](10-compiler-and-ir.md)) |
| **Partial evaluation of an interpreter**: specialise the engine to one program and get compiled code | Truffle and Graal, the Futamura projections | the engine is an interpreter over SIR-F; a backend for its own language is the first projection | not yet: a route to a native target |
| **Class-data sharing, snapshots**: start from a saved heap instead of building it | JVM CDS and AppCDS, V8 snapshots | everything pure in an app is known at build time | not yet; Bun's `--bytecode` is the same idea and was used by hand |
| **Superoptimisation**: search small rewrites and keep the cheapest that passes | Souper, STOKE | the tests and laws are the equivalence oracle | not yet |

The last column is the honest one: half of what is listed was done by hand and is not yet done by the compiler.

## Representation: the compiler picks the types

The source says what a value is; the backend picks how it is stored. The claims make most of the choice for it.

| Source says | Representation | Borrowed from | Why it is safe |
|---|---|---|---|
| `Int where 1 <= self <= 2^53` | `NonZeroU64`; `Option` of it costs no extra byte | Rust niche optimisation, Ada ranges | the claim is the bound; an unbounded `Int` keeps a big integer, never a silent wrap |
| `type Status = Open \| Done` | one byte, or one bit inside its record | C enums, bit packing | a closed sum with no data |
| `Set[Priority]`, a set of a small closed sum | a bitset: union, membership, equality in one instruction | Java `EnumSet` | every member is known at build time |
| `Text where` ASCII, at most 32 code points (`Tag`) | an inline small string, no heap; `size` is the length | small-string optimisation, `SmallVec` | the claim gives both the alphabet and the bound |
| the same short texts repeated (tags) | interned: a symbol id, compared by integer | JVM string interning, Lisp symbols | values are immutable and `==` is structural, so identity never shows |
| `Map[K, V]` whose keys only come from a counter the module owns (`next_no`) | a slab: a vector indexed by key, holes for removed entries | slab allocators, entity arrays in games | the module owns the state, so every insert is known to use the counter |
| a rule or law that reads one field of many records (`stats` reads `status`) | that field also kept as its own column | column stores, struct-of-arrays, ECS | the read sets say which fields are scanned together |
| `Id[User]`, units like `10ms` | the bare number at run time | newtypes, phantom types, unit erasure | checked in the build, gone after it ([10](10-compiler-and-ir.md)) |
| a `flow` over one field | a typestate in targets that have one: an illegal move does not compile | Rust typestate | the edges are the whole legal set |

A representation is a strategy like any other: a precondition, a cost, a record in `provenance.json`, and the store format above it unchanged. The store's bytes are fixed by the contract, never by the representation in memory.

## Beyond speed: memory, allocation, cleanup

Fast is one column. Generated code is judged on all of them, and each is a guarantee the backend states, tests and records in `provenance.json`.

| Aspect | Guarantee | Licensed by |
|---|---|---|
| Peak memory | a read-only rule never builds what its read set does not name; a stream where the store allows | read sets |
| Allocation | one pre-sized buffer for output and for the store write; parsing borrows from the read buffer instead of copying | the store format and the reply are fixed |
| No leaks by construction | `fn` and `rule` hold no resources ([34](34-hygiene.md)); a backend's handles (file, lock, temp file) live in one scope with a release on every exit: `Drop`, RAII, `ensure`, `finally` | a rule is atomic, so there is one exit for success and one for failure |
| Failure leaves nothing behind | a refused rule, a crash or an interrupt (`SIGINT`) during a write leaves the old store intact and no temp file | a rule's writes are all or none ([card](01-language-card.md)) |
| Locks always released | released on every path, before output is printed | the lock is the backend's, held for one rule |
| Start-up and binary size | counted as cost; dead rules are not emitted | the `app` lists every rule |

Each is tested on failure paths, not only on success, and measured, never assumed: peak resident memory and allocation counts per command, with the method.

## Where this goes beyond gcc

- **Aliasing is never a question.** Values are immutable and state is owned by one module, so every load and store is known. gcc spends much of its effort proving what spec-lang states.
- **Equivalence has an oracle.** A C compiler can only trust its own proofs. Here every rewrite can also be run against the program's laws over thousands of traces before it is accepted.
- **The world is closed, as native-image assumes but must check.** GraalVM needs configuration for reflection and dynamic loading; spec-lang has neither, so whole-program decisions are always safe.
- **The plan sees the whole program.** A rule's read set, the store format and the command line are one plan, so the parser, the store and the rule can be fused, which no library boundary allows.

## Limits

- Nothing on this page is implemented as a planner. Generated code today translates rule by rule; the measured gaps above are the cost.
- Checking a pass by laws is sampling: it finds wrong rewrites, it does not prove a rewrite right.
- Cost formulas without profiles are crude ([10](10-compiler-and-ir.md)). A trace workload from tests is not production traffic.
- A representation chosen from claims is only as good as the claims. `TaskNo` in the todo POC is `Int where self >= 1`, unbounded, so the narrow integer the backends use rests on a store limit the source does not state ([findings](../../pocs/todo-app/findings.md)).
- Every strategy is code in a backend, so it is also a place for a bug that reaches every project at once ([11](11-backends-and-parity.md)).
- The benchmark figures are one machine, one store shape, `hyperfine` with 20 runs; they show a gap, not its size elsewhere.
