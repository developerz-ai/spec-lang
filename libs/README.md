# Libraries

What `use <name>;` loads when no module of the project has that name ([decision 008](../docs/decisions/008-libraries-ship-their-spec.md), [009](../docs/decisions/009-one-compiler-any-language.md)). The toolchain knows no library by name: everything below is read from these files.

| Library | Adds |
|---|---|
| [`cli`](cli/README.md) | a program run from a shell: trigger `cli`, app notes `@bin` `@store`, handler `out` |
| [`game`](game/README.md) | a fixed-tick game with a window: trigger `tick`, app notes `@bin` `@window`, an app shape, handlers `screen` `keys`; offers `cli`'s `out` |
| [`raylib`](raylib/README.md) | raylib 5.5's calls as `foreign` declarations; offers `game`'s `screen` and `keys` |

## One library

| File | Holds | Read as |
|---|---|---|
| `vocab.slg` | `@allows(...) note` (an app note, or a trigger with `@allows(on)`), `type` for note parameters, `handler` with its operations, at most one `app` shape, `use other.handler;` to offer another library's handler | data: checked against, never run |
| `contract.slg` | ordinary declarations, `foreign` ones for a C library | the module `<name>`, when it declares anything |
| `pretend.slg` | `handler Name(state: mut T = start) = { handler.op(x) -> reply; }` | what the engine runs for each operation in tests |
| `real/` | one runtime per language, over the app manifest ([`real/README.md`](cli/real/README.md)) | by a build; none exists yet |
| `README.md` | what it adds, ending with Limits | people and agents |

## Where `use` looks

For `use cli;` with no module `cli` in the project, the first directory that holds `cli/vocab.slg`, in this order:

1. `libs/` beside the project, then in each directory above it, nearest first (for `pocs/todo-app/src`: `pocs/todo-app/libs`, `pocs/libs`, then this directory);
2. each directory in `SPEC_LANG_LIBS`, separated like `PATH`;
3. this directory, as the toolchain was built (temporary: until packages come over git, [decision 007](../docs/decisions/007-packages-over-git.md), libraries travel with the toolchain).

Nothing else is ever read: no install step, no code runs at load.

## Limits

- The search path is a stopgap. A lock file with a commit and a hash per library ([35](../docs/idea/35-packages.md)) replaces steps 1 to 3; until then, two checkouts can see different libraries under one name.
- Three libraries, all written for two POCs. A second program of each kind will find what they lack.
- No library has a runtime in `real/`. The todo printers in `src/crates/emit-*` still embed `cli`'s runtime, and no printer serves a `game` app.
- Notes of a library stand only on `app`, and triggers only in `@on` of a `rule`; the toolchain keeps nothing else as data yet.
