# Modules

> **Generated.** Projection `diagram --kind modules` of the program's IR, printed by the `view` crate. Do not edit: change the source and regenerate. Same source, same bytes.

**20 nodes (4 app, 9 module, 2 state, 4 target, 1 vocabulary), 30 edges (4 declares, 4 exposes, 2 owns, 4 targets, 16 uses).**

```mermaid
flowchart LR
  mod_apps_bun["apps.bun"]
  mod_apps_cpp["apps.cpp"]
  mod_apps_ruby["apps.ruby"]
  mod_apps_rust["apps.rust"]
  mod_todo_board["todo.board"]
  mod_todo_laws["todo.laws"]
  mod_todo_model["todo.model"]
  mod_todo_render["todo.render"]
  mod_todo_tests_scenarios["todo.tests.scenarios"]
  vocab_cli(("cli"))
  apps_bun_TodoBun[["TodoBun"]]
  target_bun(("bun"))
  apps_cpp_TodoCpp[["TodoCpp"]]
  target_cpp(("cpp"))
  apps_ruby_TodoRuby[["TodoRuby"]]
  target_ruby(("ruby"))
  apps_rust_TodoRust[["TodoRust"]]
  target_rust(("rust"))
  todo_board_tasks[("tasks")]
  todo_board_next_no[("next_no")]
  mod_apps_bun -.->|"uses"| vocab_cli
  mod_apps_bun -.->|"uses"| mod_todo_board
  mod_apps_cpp -.->|"uses"| vocab_cli
  mod_apps_cpp -.->|"uses"| mod_todo_board
  mod_apps_ruby -.->|"uses"| vocab_cli
  mod_apps_ruby -.->|"uses"| mod_todo_board
  mod_apps_rust -.->|"uses"| vocab_cli
  mod_apps_rust -.->|"uses"| mod_todo_board
  mod_todo_board -.->|"uses"| vocab_cli
  mod_todo_board -.->|"uses Task, TaskNo, Title, Tag, Priority, Status"| mod_todo_model
  mod_todo_board -.->|"uses line, rank"| mod_todo_render
  mod_todo_laws -.->|"uses"| mod_todo_board
  mod_todo_laws -.->|"uses TaskNo, Status"| mod_todo_model
  mod_todo_render -.->|"uses Task, Status, Priority, Tag"| mod_todo_model
  mod_todo_tests_scenarios -.->|"uses"| mod_todo_board
  mod_todo_tests_scenarios -.->|"uses Task, TaskNo, Priority"| mod_todo_model
  mod_apps_bun -->|"declares"| apps_bun_TodoBun
  apps_bun_TodoBun -->|"exposes 8 rules"| mod_todo_board
  apps_bun_TodoBun -->|"targets bin todo, store file .todo.json"| target_bun
  mod_apps_cpp -->|"declares"| apps_cpp_TodoCpp
  apps_cpp_TodoCpp -->|"exposes 8 rules"| mod_todo_board
  apps_cpp_TodoCpp -->|"targets bin todo, store file .todo.json"| target_cpp
  mod_apps_ruby -->|"declares"| apps_ruby_TodoRuby
  apps_ruby_TodoRuby -->|"exposes 8 rules"| mod_todo_board
  apps_ruby_TodoRuby -->|"targets bin todo, store file .todo.json"| target_ruby
  mod_apps_rust -->|"declares"| apps_rust_TodoRust
  apps_rust_TodoRust -->|"exposes 8 rules"| mod_todo_board
  apps_rust_TodoRust -->|"targets bin todo, store file .todo.json"| target_rust
  mod_todo_board ==>|"owns"| todo_board_tasks
  mod_todo_board ==>|"owns"| todo_board_next_no
  classDef app stroke:#db2777,stroke-width:2px
  class apps_bun_TodoBun,apps_cpp_TodoCpp,apps_ruby_TodoRuby,apps_rust_TodoRust app
  classDef state stroke:#d97706,stroke-width:2px
  class todo_board_tasks,todo_board_next_no state
  classDef target stroke:#db2777,stroke-width:2px
  class target_bun,target_cpp,target_ruby,target_rust target
```

## Limits

- Events are not drawn: the IR has no `event` yet.
- A `uses` edge lists the names imported one by one; an edge with no names is a whole-module `use`.
- Places and handlers of an `app` are not drawn; only `@target`, `@bin` and `@store` are read.
