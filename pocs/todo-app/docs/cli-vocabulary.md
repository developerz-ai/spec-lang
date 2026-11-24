# The `cli` vocabulary

> **Design, POC-local.** Nothing here runs. The [domain tour](../../../docs/idea/15-domain-tour.md) writes `use cli;` and lists what it derives; this card is the smallest set of kinds and notes the todo POC needed, written down so four backends could agree. It adds no grammar.

## What it adds

| Adds | Form | Means |
|---|---|---|
| trigger | `@on(cli("name"))` on a `rule` or `task` | the program's user runs `<bin> name ...`; parameters become arguments |
| note | `@bin("todo")` on an `app` | the executable's name |
| note | `@store(file(".todo.json"))` on an `app` | where every `state` of the app's modules lives between runs; relative to the working directory |
| effect | `out.line(text)` | one line on standard output, sent after commit |
| handler | `out.text` in a test | everything `out.line` sent in this test, each line ending in `\n` |

The local user of a program has no caller ([card](../../../docs/idea/01-language-card.md), how it runs, 4), so a `cli` rule needs no `@access`.

## Derived, never written

| From | Derived |
|---|---|
| a parameter without a default | a positional argument, in declaration order, shown `<name>` |
| `Bool = false` | a switch `--name` |
| `Option[T] = none` | `--name <value>`, at most once |
| `Set[T]` with a default | `--name <value>`, repeatable; repeats collapse |
| a sum type with no data | its variant names in lower case: `--priority high` |
| `Int where ...`, `Text where ...` | parsed, then the claim checked at the edge; a value outside it is a usage error |
| every `cli` rule | a line of `--help` |
| `fail E` | `error: E` on standard error, exit 1 |
| the `@store` | load before the rule, one lock around it, an atomic replace after it; an unreadable store exits 3 |

The exact bytes are fixed by [the projection contract](contract.md), so that every backend prints the same thing.

## Open choices this card settled

Each of these had two possible behaviours, so each was a question ([minimal and sufficient](../../../docs/idea/01-language-card.md#minimal-and-sufficient)). They are answered here and not in the language.

| Question | Answer | Why |
|---|---|---|
| `todo add buy milk`: one title or two arguments? | two arguments, so `unexpected argument "milk"` | one parameter, one argument; joining is a guess about intent |
| flag spelling for `Set[T]` named `tags` | `--tags`, repeated | the name is the parameter's; no singularising |
| a claim fails at the edge | usage error, exit 2, not a rule error | the rule never ran, so nothing was refused |
| exit code per error | one code, 1, for every rule error | the error name is on standard error; codes per name would need a registry this POC does not have |
| a number argument larger than a machine word | a `TaskNo`, so `NoTask` | `Int` is unbounded; a backend may not turn it into a usage error |

## Limits

- One POC wrote this card, for one program. A second CLI program will find what it lacks.
- Nothing in it is checked: the trigger, notes and effect parse as forms and mean nothing to any tool yet.
