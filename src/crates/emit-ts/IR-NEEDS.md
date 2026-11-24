# What the emitters need from the IR

> **State `As of 2026-11-24`.** Requests from `emit-ts` and `emit-ruby` to `spec-lang-ir`. Each is worked around locally today; the workaround is named.

| # | Need | Why | Today |
|---|---|---|---|
| 1 | A type on every expression | `x.size` of a text, a list and a map are three different target expressions; `==` on records is not `===` | `emit_ts::sem::infer` re-derives types from declared ones; `emit-ruby` depends on `emit-ts` to share it. It belongs in the IR, or in a crate both use |
| 2 | Core library calls resolved to an operation | the same reason: `Method { name: "filter" }` is a string | `sem::tree::CoreOp`, matched on the receiver's type |
| 3 | Coercions made explicit | a base value entering a claimed type must be checked or proved; `Int` literals adapt | `TKind::Claim`, `TKind::Widen` inserted by `infer` |
| 4 | Read and write sets per rule | lock mode, drafts, projection pushdown | the write set is scanned from the body; `rule_set_uses` was added to the IR later and is not used yet. There is no read set, so there is no projection |
| 5 | State laws marked | a law that holds in every state can be checked when a store is read | `sem::app` recognises `t.states.all(w => p)` by shape |
| 6 | The store schema as a derived fact of the `cli` vocabulary | four backends must agree on the bytes | `sem::app::Codec`, by a rule this backend wrote down (README, open choices) |
| 7 | Exhaustiveness and claim proofs | a match with a missing arm, or `no + 1 >= 1`, are compile-time facts | `infer` checks exhaustiveness itself; claims are checked at run time |
| 8 | A source path that is not a local path | headers and `provenance.json` must not hold one | the path is rebuilt from the module name: `todo.board` is `todo/board.slg` |

## Limits

- This list is what one program needed. A second program will add to it.
