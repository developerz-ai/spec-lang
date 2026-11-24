# `game` runtimes

One directory per language: `rust`, `ts`, `ruby`, `cpp` ([decision 009](../../../docs/decisions/009-one-compiler-any-language.md)).

## What a runtime implements

It reads the **app manifest** `spec-lang build` emits beside the program's code: the app, its notes with their arguments by parameter name, its six fields with the declarations they name, and each rule's triggers.

| From the manifest | The runtime does |
|---|---|
| `start` and its one parameter | `--level <n>`, parsed as that parameter's type |
| `tick`, with trigger `tick(rate)` | one call per tick at that rate; a refusal ends the run |
| `input` | read once per tick in `play`; in `run`, `draw` and `shot` the controls come from a script on standard input |
| `frame` | draw one frame in `play` and `shot` |
| `dump`, `report` | the frame as text for `draw`; one line after `run` |
| `@window(width, height, title)`, `@bin(name)` | the window `play` and `shot` open; the executable's name |
| handler `out` | as in `cli`: lines after the rule commits |
| handlers `screen`, `keys` | nothing of its own: a drawing or keyboard library's runtime (`raylib`) implements the calls that state them |

The commands and their bytes are the platformer's [contract](../../../pocs/platformer/docs/contract.md); its parity suite is the judge.

## Limits

- The four runtimes were lifted from one game's hand-written backends; a second game will find what they lack.
