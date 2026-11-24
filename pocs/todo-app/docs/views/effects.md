# Effects

> **Generated.** Projection `diagram --kind effects` of the program's IR, printed by the `view` crate. Do not edit: change the source and regenerate. Same source, same bytes.

**11 nodes (1 effect, 8 rule, 2 state), 23 edges (8 emits, 8 reads, 7 writes).** A state with a red border is written by one rule and read or written by another.

```mermaid
flowchart LR
  subgraph g_todo_board["todo.board"]
    todo_board_add(["add"])
    todo_board_list(["list"])
    todo_board_done(["done"])
    todo_board_reopen(["reopen"])
    todo_board_rename(["rename"])
    todo_board_remove(["remove"])
    todo_board_clear(["clear"])
    todo_board_stats(["stats"])
    todo_board_tasks[("tasks")]
    todo_board_next_no[("next_no")]
  end
  fx_out>"out"]
  todo_board_add -.->|"reads"| todo_board_next_no
  todo_board_add ==>|"writes"| todo_board_tasks
  todo_board_add ==>|"writes"| todo_board_next_no
  todo_board_add -->|"emits line"| fx_out
  todo_board_list -.->|"reads"| todo_board_tasks
  todo_board_list -->|"emits line"| fx_out
  todo_board_done -.->|"reads"| todo_board_tasks
  todo_board_done ==>|"writes"| todo_board_tasks
  todo_board_done -->|"emits line"| fx_out
  todo_board_reopen -.->|"reads"| todo_board_tasks
  todo_board_reopen ==>|"writes"| todo_board_tasks
  todo_board_reopen -->|"emits line"| fx_out
  todo_board_rename -.->|"reads"| todo_board_tasks
  todo_board_rename ==>|"writes"| todo_board_tasks
  todo_board_rename -->|"emits line"| fx_out
  todo_board_remove -.->|"reads"| todo_board_tasks
  todo_board_remove ==>|"writes"| todo_board_tasks
  todo_board_remove -->|"emits line"| fx_out
  todo_board_clear -.->|"reads"| todo_board_tasks
  todo_board_clear ==>|"writes"| todo_board_tasks
  todo_board_clear -->|"emits line"| fx_out
  todo_board_stats -.->|"reads"| todo_board_tasks
  todo_board_stats -->|"emits line"| fx_out
  classDef effect stroke:#6b7280,stroke-width:1px,stroke-dasharray:4 3
  class fx_out effect
  classDef rule stroke:#2563eb,stroke-width:2px
  class todo_board_add,todo_board_list,todo_board_done,todo_board_reopen,todo_board_rename,todo_board_remove,todo_board_clear,todo_board_stats rule
  classDef state stroke:#d97706,stroke-width:2px
  class todo_board_tasks,todo_board_next_no state
  classDef hot stroke:#dc2626,stroke-width:4px
  class todo_board_tasks,todo_board_next_no hot
```

| Rule | `@uses` | Reads | Writes | Outside | Not in `@uses` | In `@uses`, not found |
|---|---|---|---|---|---|---|
| `todo.board.add` | `tasks.write`, `next_no.write`, `out.write` | `todo.board.next_no` | `todo.board.tasks`, `todo.board.next_no` | `out.line` | - | - |
| `todo.board.list` | `tasks.read`, `out.write` | `todo.board.tasks` | - | `out.line` | - | - |
| `todo.board.done` | `tasks.write`, `out.write` | `todo.board.tasks` | `todo.board.tasks` | `out.line` | - | - |
| `todo.board.reopen` | `tasks.write`, `out.write` | `todo.board.tasks` | `todo.board.tasks` | `out.line` | - | - |
| `todo.board.rename` | `tasks.write`, `out.write` | `todo.board.tasks` | `todo.board.tasks` | `out.line` | - | - |
| `todo.board.remove` | `tasks.write`, `out.write` | `todo.board.tasks` | `todo.board.tasks` | `out.line` | - | - |
| `todo.board.clear` | `tasks.write`, `out.write` | `todo.board.tasks` | `todo.board.tasks` | `out.line` | - | - |
| `todo.board.stats` | `tasks.read`, `out.write` | `todo.board.tasks` | - | `out.line` | - | - |

## Rules whose order matters

**33 pairs** (8 nodes, 33 edges). One rule writes a state the other reads or writes. Each rule is atomic, so nothing is torn; the result depends on which commits first. A loop on one rule is two calls of it.

```mermaid
flowchart LR
  subgraph g_todo_board["todo.board"]
    todo_board_add(["add"])
    todo_board_list(["list"])
    todo_board_done(["done"])
    todo_board_reopen(["reopen"])
    todo_board_rename(["rename"])
    todo_board_remove(["remove"])
    todo_board_clear(["clear"])
    todo_board_stats(["stats"])
  end
  todo_board_add <-->|"can race on tasks, next_no"| todo_board_add
  todo_board_add <-->|"can race on tasks"| todo_board_list
  todo_board_add <-->|"can race on tasks"| todo_board_done
  todo_board_add <-->|"can race on tasks"| todo_board_reopen
  todo_board_add <-->|"can race on tasks"| todo_board_rename
  todo_board_add <-->|"can race on tasks"| todo_board_remove
  todo_board_add <-->|"can race on tasks"| todo_board_clear
  todo_board_add <-->|"can race on tasks"| todo_board_stats
  todo_board_list <-->|"can race on tasks"| todo_board_done
  todo_board_list <-->|"can race on tasks"| todo_board_reopen
  todo_board_list <-->|"can race on tasks"| todo_board_rename
  todo_board_list <-->|"can race on tasks"| todo_board_remove
  todo_board_list <-->|"can race on tasks"| todo_board_clear
  todo_board_done <-->|"can race on tasks"| todo_board_done
  todo_board_done <-->|"can race on tasks"| todo_board_reopen
  todo_board_done <-->|"can race on tasks"| todo_board_rename
  todo_board_done <-->|"can race on tasks"| todo_board_remove
  todo_board_done <-->|"can race on tasks"| todo_board_clear
  todo_board_done <-->|"can race on tasks"| todo_board_stats
  todo_board_reopen <-->|"can race on tasks"| todo_board_reopen
  todo_board_reopen <-->|"can race on tasks"| todo_board_rename
  todo_board_reopen <-->|"can race on tasks"| todo_board_remove
  todo_board_reopen <-->|"can race on tasks"| todo_board_clear
  todo_board_reopen <-->|"can race on tasks"| todo_board_stats
  todo_board_rename <-->|"can race on tasks"| todo_board_rename
  todo_board_rename <-->|"can race on tasks"| todo_board_remove
  todo_board_rename <-->|"can race on tasks"| todo_board_clear
  todo_board_rename <-->|"can race on tasks"| todo_board_stats
  todo_board_remove <-->|"can race on tasks"| todo_board_remove
  todo_board_remove <-->|"can race on tasks"| todo_board_clear
  todo_board_remove <-->|"can race on tasks"| todo_board_stats
  todo_board_clear <-->|"can race on tasks"| todo_board_clear
  todo_board_clear <-->|"can race on tasks"| todo_board_stats
  classDef rule stroke:#2563eb,stroke-width:2px
  class todo_board_add,todo_board_list,todo_board_done,todo_board_reopen,todo_board_rename,todo_board_remove,todo_board_clear,todo_board_stats rule
```

| Rule | Rule | Shared state |
|---|---|---|
| `todo.board.add` | `todo.board.add` | `todo.board.tasks`, `todo.board.next_no` |
| `todo.board.add` | `todo.board.list` | `todo.board.tasks` |
| `todo.board.add` | `todo.board.done` | `todo.board.tasks` |
| `todo.board.add` | `todo.board.reopen` | `todo.board.tasks` |
| `todo.board.add` | `todo.board.rename` | `todo.board.tasks` |
| `todo.board.add` | `todo.board.remove` | `todo.board.tasks` |
| `todo.board.add` | `todo.board.clear` | `todo.board.tasks` |
| `todo.board.add` | `todo.board.stats` | `todo.board.tasks` |
| `todo.board.list` | `todo.board.done` | `todo.board.tasks` |
| `todo.board.list` | `todo.board.reopen` | `todo.board.tasks` |
| `todo.board.list` | `todo.board.rename` | `todo.board.tasks` |
| `todo.board.list` | `todo.board.remove` | `todo.board.tasks` |
| `todo.board.list` | `todo.board.clear` | `todo.board.tasks` |
| `todo.board.done` | `todo.board.done` | `todo.board.tasks` |
| `todo.board.done` | `todo.board.reopen` | `todo.board.tasks` |
| `todo.board.done` | `todo.board.rename` | `todo.board.tasks` |
| `todo.board.done` | `todo.board.remove` | `todo.board.tasks` |
| `todo.board.done` | `todo.board.clear` | `todo.board.tasks` |
| `todo.board.done` | `todo.board.stats` | `todo.board.tasks` |
| `todo.board.reopen` | `todo.board.reopen` | `todo.board.tasks` |
| `todo.board.reopen` | `todo.board.rename` | `todo.board.tasks` |
| `todo.board.reopen` | `todo.board.remove` | `todo.board.tasks` |
| `todo.board.reopen` | `todo.board.clear` | `todo.board.tasks` |
| `todo.board.reopen` | `todo.board.stats` | `todo.board.tasks` |
| `todo.board.rename` | `todo.board.rename` | `todo.board.tasks` |
| `todo.board.rename` | `todo.board.remove` | `todo.board.tasks` |
| `todo.board.rename` | `todo.board.clear` | `todo.board.tasks` |
| `todo.board.rename` | `todo.board.stats` | `todo.board.tasks` |
| `todo.board.remove` | `todo.board.remove` | `todo.board.tasks` |
| `todo.board.remove` | `todo.board.clear` | `todo.board.tasks` |
| `todo.board.remove` | `todo.board.stats` | `todo.board.tasks` |
| `todo.board.clear` | `todo.board.clear` | `todo.board.tasks` |
| `todo.board.clear` | `todo.board.stats` | `todo.board.tasks` |

## Limits

- Sets are per state, not per key: two rules that touch different entries of one map are still listed as a pair.
- Sets are what a rule can touch on some path, not what every call touches.
- `out` is left out of the pairs: lines are sent after commit.
- `@uses` is compared by name; the IR records it and does not check it.
