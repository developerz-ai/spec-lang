# `cli` runtimes

One directory per language: `rust`, `ts`, `ruby`, `cpp` ([decision 009](../../../docs/decisions/009-one-compiler-any-language.md)).

## What a runtime implements

It reads the **app manifest** `spec-lang build` emits beside the program's code: the app, its library notes with their arguments by parameter name, its rules with their parameters, types and triggers. The program's code is printed from the LIR; the runtime adds:

| From the manifest | The runtime does |
|---|---|
| each rule with trigger `cli(command)` | a command `<bin> command ...`; parameters become arguments by the derivation table of the [card](../../../pocs/todo-app/docs/cli-vocabulary.md); `--help` lists every command |
| `@bin(name)` | the executable's name |
| `@store(file(path))` | load every state before the rule, one lock around it, an atomic replace after it; an unreadable store exits 3 |
| a refusal `fail E` | `error: E` on standard error, exit 1; a usage error exits 2 |
| handler `out`: `line(text)` | one line on standard output, sent only after the rule commits |
| handler `out`: `text` | in the printed tests, every line sent in that test, each ending in a newline |

The bytes are the [contract](../../../pocs/todo-app/docs/contract.md)'s. A runtime is right when the program's own checks, printed with it, report what `spec-lang test` reports, check by check, and when the POC's parity suite passes.

## Limits

- The manifest's shape is each printer's own data; the LIR fixes what it carries, not how a runtime reads it. The four runtimes were lifted from the todo POC's hand-written backends, so a second `cli` program will find what they lack.
