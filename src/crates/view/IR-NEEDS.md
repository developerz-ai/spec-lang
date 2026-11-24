# What the views need from the IR

> **Open requests.** Each row is something `spec-lang-ir` did not provide on 2026-11-24, what the view crate does without it, and what would change with it. Nothing here blocks the todo program.

| Need | Without it | With it |
|---|---|---|
| `flow` declarations and their `@edge(A -> B, via:, by:, after:)` | `state::FlowEdge` is passed in as data; nothing fills it, so only machines read off rules are drawn | declared machines drawn first, and a rule's inferred move checked against them |
| `event` declarations, `emit` and `@on(event)` | the modules view has no event edges; the call graph has no `emits`/`hears` between rules | "events" in the modules view, as [docs/idea/10](../../../docs/idea/10-compiler-and-ir.md) lists |
| The alias a `Trace[...]` parameter was written with (`Trace[Change]`) | `Ty::Trace` holds the expanded rules, so a law has no reference to `todo.laws.Change` and its impact is empty | impact of a rule-set type lists the laws over it |
| The state a `Prop` on a world resolves to (`w.tasks`, `it.after.next_no` in a law) | a law depends on state only through the rules it traces | a law that reads `next_no` is listed for `next_no` directly, and not for states it never reads |
| The type of an expression | a place's type is rebuilt from the state's declared type, step by step (`Facts::place_ty`); a move through a local or a `with` is not found | state moves found wherever a sum-typed value is written |
| Read sets per key (point and predicate reads, as SIR-F is meant to carry) | effects are per state: two rules on different entries of one map are listed as a pair | the pairs that can really collide |
| Checked `@uses` | the view compares `@uses` with the body by name and reports both differences | the comparison is the compiler's, the view only shows it |
| `Trigger` kinds beyond `Cli`, `Store` beyond `File`, `Effect` beyond `OutLine` | matched exhaustively, so a new variant fails to compile here until it is drawn | n/a |
| A `&'a` visitor (`walk_expr` hands out `&Expr` with no lifetime tied to the program) | the crate keeps its own `facts::walk`, which lets a caller collect the nodes it visits | one walker for every crate |
| Vocabulary kinds as retained nodes with their expansion | nothing to show for `view --as expanded` | the `expanded` projection |

## Limits

- This list is what the views wanted while being written against one program; other programs will add rows.
- A row says what is missing, not how the IR should hold it.
