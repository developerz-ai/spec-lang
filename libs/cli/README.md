# `cli`

A program its user runs from a shell: one command per rule, its parameters the arguments, the world kept in a file between runs. The form is in [`vocab.slg`](vocab.slg); what each piece means, byte for byte, is the todo POC's [vocabulary card](../../pocs/todo-app/docs/cli-vocabulary.md) and [contract](../../pocs/todo-app/docs/contract.md), which a runtime in `real/` must follow.

| Adds | Form | Checked here |
|---|---|---|
| trigger | `@on(cli("add"))` on a rule | one text argument |
| app note | `@bin("todo")` | one text |
| app note | `@store(file(".todo.json"))` | a `Store`, whose one variant is `file(path)` |
| handler `out` | `out.line(text)` in a rule; effect `out.write` | a rule sends it, a `fn` cannot |
| test handler | `out.text` in a test | read by tests only, in any module |

`contract.slg` is empty: arguments, the store and standard output are the runtime's, not calls a program makes. `pretend.slg` keeps the lines a test sent: a rule refused rolls its lines back with its writes, and each test starts with none.

## Limits

- One POC wrote it, for one program. Derived parsing (flags, sets, sums as words) is a table in the POC's card, not a declaration here: the toolchain checks a rule's trigger, not that its parameters have a command-line form.
- `@store` has one variant. Another kind of store is a new variant here and a new branch in every runtime.
- The todo printers in `src/crates/emit-*` read `@bin`, `@store`, `cli(...)` and `out` by these names; they are this library's runtime, not yet moved to `real/`.
