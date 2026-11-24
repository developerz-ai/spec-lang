# Views of todo-app

> **Generated.** Every file in this folder is a projection of the program's IR, printed by the `view` crate. Do not edit: change the source and regenerate. Same source, same bytes, so a stale file is a failing check.

The program: 9 modules, 7 types, 2 states, 9 functions, 8 rules, 10 tests, 7 laws, 4 apps.

| View | Read it | Also | Diagrams | Nodes | Edges |
|---|---|---|---|---|---|
| class | [class.md](class.md) | [puml](class.puml), [json](class.json) | 1 | 8 | 6 |
| state | [state.md](state.md) | [puml](state.puml), [json](state.json) | 1 | 3 | 5 |
| sequence | [sequence.md](sequence.md) | [puml](sequence.puml), [json](sequence.json) | 10 | 47 | 83 |
| modules | [modules.md](modules.md) | [json](modules.json) | 1 | 20 | 30 |
| effects | [effects.md](effects.md) | [json](effects.json) | 1 | 11 | 23 |
| calls | [calls.md](calls.md) | [json](calls.json) | 1 | 25 | 44 |

More in the same pages: the effects view also lists **33 pairs of rules whose order matters**; the call graph with tests and laws has **43 nodes, 124 edges**; [impact.md](impact.md) ([json](impact.json)) has one graph for each of the 26 declarations something depends on; [pseudo.md](pseudo.md) is every function and rule as pseudocode.

The explorer is [site/index.html](site/index.html): 65 files, one page per module and per declaration with its source, notes, diagrams, what it names, what depends on it and its tests and laws. Open it from a checkout; it loads one pinned script to draw Mermaid (`https://cdn.jsdelivr.net/npm/mermaid@11.17.2/dist/mermaid.min.js`) and shows the diagram text without it.

Regenerate:

```bash
cd src && VIEW_BLESS=1 cargo test -p view --test golden
```

## Limits

- Every view is read off the declarations without running a rule: read and write sets are what a rule can touch, sequences are a test's written steps.
- Counts are of the diagram as printed. They change when the source does, and that diff is the point.
- Each page ends with what its view does not show.
