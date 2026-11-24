---
name: slg
description: Use when writing or editing spec-lang source anywhere in this repo: a ```slg fence in docs/, a .slg file under pocs/, or an example in a decision record.
---

# Writing spec-lang

1. The card is the language: `docs/idea/01-language-card.md`. Read it if it is not in context. Nothing else overrides it.
2. One shape: `{@note(args)} [pub] kind Name[generics](data) -> Result = body`. A new idea is a kind or a note, never new syntax.
3. Notes carry every claim and policy; `( )` carries only data you invent. `@why` on anything whose reason is not obvious.
4. Two possible behaviours is a question (`E-OPEN`), never a guess. Write the question in the doc's Limits or in a decision.
5. Not source? Use another fence language (`text`), never `slg`: every `slg` fence must parse.
6. `bin/slg-check <file>` · fail → fix the form. Changing `docs/idea/25-grammar.md` needs a decision record.
7. A program you can run end to end belongs in `pocs/<name>/`; copy the layout of `pocs/todo-app/`.
