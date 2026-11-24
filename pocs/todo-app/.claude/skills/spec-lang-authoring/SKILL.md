---
name: spec-lang-authoring
description: Use when editing any .slg file under pocs/todo-app/src/, adding a rule, type, law or test to the todo program, or when a parity case and the spec disagree.
---

# Changing the todo program

1. Read `../../docs/idea/01-language-card.md` if it is not in context. The card wins over every other doc.
2. Edit `src/todo/<module>.slg`. One declaration shape: `{@note(args)} [pub] kind Name(data) -> Result = body`. No new syntax, ever.
3. A new rule: `@on(cli("name"))`, `@uses(...)` with every state it reads or writes plus `out.write`, guards in the order errors must win. Add it to every `src/apps/*.slg`.
4. A new claim on data: a `fn is_x` with `@cases`, then `type X = T where is_x(self)`.
5. Every behaviour gets a `test` in `src/todo/tests/scenarios.slg`; every invariant a `law` in `src/todo/laws.slg`.
6. `../../bin/slg-check src` · fail → fix the form, never the grammar.
7. Surface changed (output bytes, flags, errors, store)? → `docs/contract.md` and `docs/cli-vocabulary.md`, then cases in `parity/cases/`.
8. `/project <backend>` for all four, or brief four `projector` agents in parallel, one folder each.
9. `bin/check` · red → read which side is wrong: spec, contract, or backend. Fix that side only.
