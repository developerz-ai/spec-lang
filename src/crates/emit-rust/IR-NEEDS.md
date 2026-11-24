# What the emitters need from the IR

> **Requests, not facts about the IR.** Each row is something `emit-rust` and `emit-cpp` compute themselves today, in `src/front/`, because `spec-lang-ir` does not provide it. The coordinator relays these; nothing here edits the ir crate.

| # | Need | Stopgap here | Why it belongs in the IR |
|---|---|---|---|
| 1 | A type on every expression, core-library members resolved (`size`, `filter`, `keys`), defaults filled in, refinement changes explicit | `front/typer.rs` builds `front::typed` from `ir::Expr` | every backend and the interpreter need the same answer; this is SIR-F in `docs/idea/10-compiler-and-ir.md` |
| 2 | Each rule's write set and read set | `front/facts.rs`, from assignments and removals | the planner's read sets (`docs/idea/11-backends-and-parity.md`, atomic rules) are SIR-F facts |
| 3 | Map invariants that follow from every writer: "`m` is keyed by field `f` of its values" | `front/facts.rs`, `keyed` | licenses the store schema and the `retain` strategy; both backends use it. The todo spec now states it as the law `stored_under_its_number`; the inference should be checked against such a law rather than replace it |
| 4 | Laws that are state invariants (`t.states.all(w => P(w))`) marked as such | `front/facts.rs`, `invariants` | a reader of an outside world (the store) must check them |
| 5 | The `cli` vocabulary's derived surface (positional, switch, single, set; help placeholders) | `front/cli.rs` | it is the vocabulary's expansion, not a backend's choice (`docs/idea/10-compiler-and-ir.md`, retained kinds) |
| 6 | `Program::app_id(&AppDecl)`, or `emit` taking an `AppId` | `lib.rs` finds the app by its `DeclInfo` | tables are addressed by id |
| 7 | Paths in `SourceMap` relative to the project root from `load_project` | tests call `load_sources` with `src/...` paths | provenance and headers must not carry a local directory |
| 8 | A span per `@uses` entry | diagnostics point at the rule | an `E-EFFECT-UNDECLARED` fix should be an edit of the `@uses` list |
| 9 | Codes `E-PLAN-REFUSED` and `E-NO-STRATEGY` in `crates/diag/codes.md` | reported as `E-UNSUPPORTED` with a note | `docs/idea/10` and `11` name them; a refusal by a backend is not a missing feature |
| 10 | A rule in the `cli` vocabulary for the variable that replaces `@store`'s path | derived as `<BIN>_STORE` (gives `TODO_STORE`) | the contract names `TODO_STORE`; nothing written derives it |
| 11 | A claim on `Int` the store can carry (`2^53`) as a stated refusal | the store writer refuses an `Int` above `2^53` as `full` | `pocs/todo-app/findings.md`, spec error 3 |
| 12 | Read sets per field (`m[_].status`), for projection pushdown | `front/facts.rs`, `projection` | the planner's read sets, at field grain |

When rows 1 to 5 land in the ir crate, `front/` shrinks to nothing and `emit-cpp` drops its dependency on `emit-rust`.

## Limits

- Written against the ir crate as of 2026-11-24; its API was still changing while this was written.
- Rows 1 to 5 cover only the subset the todo program uses; a larger program will add rows.
