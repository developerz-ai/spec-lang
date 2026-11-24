# Build layout

> **Design.** Nothing here runs yet. Where generated software goes, and why it is not one output file beside each source file.

TypeScript puts `a.js` beside `a.ts`, or mirrors the tree in an output folder. That works because one file becomes one file. Here one rule may become a table change, a route, a client, a job and a test, in three languages. So output follows **the target**, not the source tree.

## Two layouts

| | Standalone | Embedded |
|---|---|---|
| When | the app is all spec-lang | one module inside an existing app ([12](12-frameworks.md)) |
| Output | `out/<app>/`, a complete project in the target's own layout | real files where that framework looks for them |
| In git | no: rebuilt from source and lock file, like `dist/` | yes at first, so reviewers and the framework's own tools see them |
| Marked by | the folder | a header with declaration and hash |
| Edited by hand | never; overwritten | never; a hash mismatch fails `land` |

```text
shop/
  sales/orders.slg          source: the only thing anyone edits
  stock/items.slg
  apps/web.slg              picks targets
  apps/mobile.slg
  spec-lang.lock            vocabularies, backends, plan
  out/                      ignored by git
    web/                    an Ultimate project: routes, entities, jobs, tests
    mobile-ios/             a SwiftUI project
    mobile-android/         a Compose project
    db/                     schema and migrations
    map.json                declaration <-> emitted spans
```

Each project under `out/` opens in its own tools and builds with them. Nothing in it imports spec-lang.

## Compared with TypeScript

| TypeScript | spec-lang |
|---|---|
| `tsc` | `spec-lang build` |
| `a.ts` to `a.js` | a declaration to every place it lands, listed in `map.json` |
| `.d.ts` | a generated typed client per consumer language |
| `tsconfig.json` | the `app` declaration; no second config language |
| `outDir` | `out/<app>/`, fixed |
| source maps | the same, per target: JavaScript maps, `#line`, debug info |
| `tsc --watch` | the interpreter reruns tests on each edit; `build --watch` only when a target is open |
| type-check, then emit | **test the logic, then emit** |
| JavaScript files in the same project | `foreign` contracts ([31](31-dropping-down.md)) |

## What runs when

| Moment | Runs | Emits |
|---|---|---|
| writing logic | interpreter and affected tests | nothing |
| looking at a screen or a game | `build --watch` for that one app | changed files; the target's own hot reload takes over |
| landing | all tests, guarantee diff | embedded files only |
| CI | logic, then generate, compile, parity ([22](22-lifecycle.md)) | everything, per app |

Most edits never reach the second row. That is the point: a wrong rule is found before any toolchain starts.

## Small changes stay small

- The unit is the declaration. Changing one rule re-emits only the files whose content includes it.
- Names in output come from the declaration's identity and its role, so adding a rule does not rename its neighbours.
- A file that would be byte-identical is not rewritten, so the target's incremental build sees no change.
- Two modules never write one file, except the small registries a framework insists on. Those are sorted, one line per entry.
- Output order, formatting and names are fixed. Same source, same lock, same bytes.

## Finding the way back

`spec-lang view --from out/web/api/orders.ts:88` prints the declaration, the strategy and the claims behind that line. Stack traces and profiles are mapped before anyone reads them ([10](10-compiler-and-ir.md)).

## Limits

- Parts of a framework's layout are conventions nobody wrote down; a backend has to learn them and may break on an upgrade.
- Committed output in the embedded layout makes diffs longer. It can stop once a backend's parity record is trusted, and that is a judgment.
- Compilers of the target languages are still as slow as they are. This doc reduces how often they run, not how long they take.
- A change to a widely used type re-emits much of every app.
