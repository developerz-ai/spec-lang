# The `game` vocabulary

> **Design, POC-local.** The smallest set of kinds and notes the platformer needed, written down so four backends could agree. It adds no grammar. [Games](../../../docs/idea/13-games.md) sketches `use game;`; this card is the part a fixed-tick, single-player game with a window needs. The toolchain loads it as a library from [`libs/game/vocab.slg`](../../../libs/game/vocab.slg) ([009](../../../docs/decisions/009-one-compiler-any-language.md)); the raylib calls are also a library, [`libs/raylib`](../../../libs/raylib/README.md), though this POC still declares its own in `src/hop/screen.slg`.

## What it adds

| Adds | Form | Means |
|---|---|---|
| trigger | `@on(tick(60Hz))` on a `rule` | called once per tick of play, at that rate, with the controls of that tick |
| note | `@bin("skyhop")` on an `app` | the executable's name |
| note | `@window(960, 540, "Skyhop")` on an `app` | the window `play` opens: width and height in pixels, title |
| effect | `out.line(text)` | one line on standard output, sent after the rule commits |
| effect | `screen.write` | draws on the window; only `foreign` declarations perform it |
| effect | `keys.read` | reads the keyboard; only `foreign` declarations perform it |
| app data | `start`, `tick`, `input`, `frame`, `dump`, `report` | the rule that starts a level, the tick rule, the `fn` that reads the controls, the rule that draws a frame, the `fn` that writes a frame as text, the `fn` that sums up a run |

## Derived, never written

| From | Derived |
|---|---|
| the app's `start` rule and its one parameter | `--level <n>`, parsed as that parameter's type |
| the `tick` rule's one parameter, a record of `Bool` fields | the input script: one letter per field, the field's first letter in upper case (`Pad`: `L R F B J`); first letters must differ |
| `tick`, `input`, `frame` | `play`: a window, a fixed 60 Hz loop of read controls, one tick, draw |
| `tick`, `report` | `run`: a script of controls on standard input, one tick per line count, every `out.line`, then the report |
| `tick`, `dump` | `draw`: the same, silent, then the frame as text |
| `tick`, `frame` | `shot`: the same, silent, then the frame drawn off screen and its pixels hashed |
| a guard that refuses a tick | the end of the run: `run`, `draw` and `shot` stop reading input |
| `foreign c "raylib#Name"` | a call into the C library, through the backend's own foreign function interface; the conversions at the edge are in [the contract](contract.md) §5 |

The exact bytes are fixed by [the projection contract](contract.md).

## Open choices this card settled

| Question | Answer | Why |
|---|---|---|
| Where controls come from when there is no keyboard | a script on standard input | the tick rule's parameter is the whole input; a script is that parameter, repeated |
| One tick per frame, or as many as real time asks | one per frame at a fixed 60 frames per second | the simulation never reads a clock; a slow frame slows the game and never changes it |
| What a refused tick means to the loop | the run is over: `run` reports, `play` keeps drawing the last frame | `Over` is the only refusal the rule has |
| Where meshes and shaders live | files the vocabulary loads by name: `assets/out/<asset>.glb`, `assets/shaders/lit.{vs,fs}` | the card has no form for a resource; see findings |

## Limits

- One POC wrote this card, for one program. A second game will find what it lacks: sound, more than one player, a menu, saving.
- The edge conversions (sub-metres to `float`, a record to a C struct) live in the contract because the card has no `f32` and no C struct layout. That is a finding, not a design.
