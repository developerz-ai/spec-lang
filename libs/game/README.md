# `game`

A fixed-tick, single-player game with a window. The form is in [`vocab.slg`](vocab.slg); the meaning is the platformer POC's [vocabulary card](../../pocs/platformer/docs/game-vocabulary.md) and [contract](../../pocs/platformer/docs/contract.md).

| Adds | Form | Checked here |
|---|---|---|
| trigger | `@on(tick(60Hz))` on a rule | a whole number from `1Hz` to `1000Hz` |
| app note | `@bin("skyhop")` | one text |
| app note | `@window(960, 540, "Skyhop")` | width and height from 1 to 16384, a title |
| app shape | `app A(start: .., tick: .., input: .., frame: .., dump: .., report: ..)` | each names a rule or a `fn` as `app Game(...)` says; the rule named by `tick` has `@on(tick(..))` |
| handler `screen` | effect `screen.write` | only `foreign` declarations state it; a `fn` cannot have it |
| handler `keys` | effect `keys.read` | only `foreign` declarations state it |
| handler `out` | offered from `cli` (`use cli.out;`) | as in `cli` |

`contract.slg` is empty: drawing and the keyboard are another library's calls, [`raylib`](../raylib/README.md) among them. `pretend.slg` is empty: a check that reaches `screen` or `keys` reaches a `foreign` body and is blocked.

## Limits

- One POC wrote it. Sound, several players, menus and saving are not here.
- `1000Hz` and `16384` are bounds chosen here, not measured needs; the platformer uses `60Hz` and `960 x 540`.
- What `play`, `run`, `draw` and `shot` do with the six fields is the runtime's, and no runtime exists: `spec-lang build` refuses a `game` app.
- `@bin` is declared here and in `cli` with the same form; a module that uses both cannot write it (`E-NOTE-AMBIG`).
