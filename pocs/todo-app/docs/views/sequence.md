# Sequence diagrams

> **Generated.** Projection `diagram --kind sequence` of the program's IR, printed by the `view` crate. Do not edit: change the source and regenerate. Same source, same bytes.

**10 tests.** One diagram per `test`, read off its body; nothing was run.

| Test | Participants | Messages |
|---|---|---|
| `todo.board.three_open_one_done` | 3 | 0 |
| `todo.tests.scenarios.numbers_start_at_one` | 5 | 10 |
| `todo.tests.scenarios.numbers_are_never_reused` | 5 | 19 |
| `todo.tests.scenarios.empty_board_says_so` | 4 | 8 |
| `todo.tests.scenarios.list_ranks_urgent_first` | 5 | 4 |
| `todo.tests.scenarios.list_all_and_by_tag` | 5 | 4 |
| `todo.tests.scenarios.done_and_reopen_move_once` | 5 | 14 |
| `todo.tests.scenarios.rename_keeps_the_rest` | 5 | 4 |
| `todo.tests.scenarios.clear_removes_only_done` | 5 | 8 |
| `todo.tests.scenarios.unknown_numbers_are_refused` | 5 | 12 |

## `todo.board.three_open_one_done`

```mermaid
sequenceDiagram
  actor caller as three_open_one_done
  participant s_tasks as tasks
  participant s_next_no as next_no
  Note over s_tasks: given Map( 1 -#gt; Task(1, title: #quot;low one#quot;, priority: Low), 2 -#gt; Task(2, title: #quot;plain#quot;), 3 -#gt; Ta…
  Note over s_next_no: given 5
```

## `todo.tests.scenarios.numbers_start_at_one`

```mermaid
sequenceDiagram
  actor caller as numbers_start_at_one
  participant m_todo_board as todo.board
  participant s_tasks as tasks
  participant s_next_no as next_no
  participant x_out as out
  caller->>m_todo_board: board.add(#quot;milk#quot;)
  m_todo_board->>s_tasks: write
  m_todo_board->>s_next_no: read, write
  m_todo_board->>x_out: line
  m_todo_board-->>caller: 1
  caller->>m_todo_board: board.add(#quot;bread#quot;, tags: Set(#quot;shop#quot;, #quot;home#quot;))
  m_todo_board->>s_tasks: write
  m_todo_board->>s_next_no: read, write
  m_todo_board->>x_out: line
  m_todo_board-->>caller: 2
  Note over caller: assert out.text == #quot;added 1#92;nadded 2#92;n#quot;
```

## `todo.tests.scenarios.numbers_are_never_reused`

```mermaid
sequenceDiagram
  actor caller as numbers_are_never_reused
  participant m_todo_board as todo.board
  participant s_tasks as tasks
  participant s_next_no as next_no
  participant x_out as out
  caller->>m_todo_board: board.add(#quot;a#quot;)
  m_todo_board->>s_tasks: write
  m_todo_board->>s_next_no: read, write
  m_todo_board->>x_out: line
  m_todo_board-->>caller: 1
  caller->>m_todo_board: board.add(#quot;b#quot;)
  m_todo_board->>s_tasks: write
  m_todo_board->>s_next_no: read, write
  m_todo_board->>x_out: line
  m_todo_board-->>caller: 2
  caller->>m_todo_board: board.remove(2)
  m_todo_board->>s_tasks: read, write
  m_todo_board->>x_out: line
  m_todo_board-->>caller: ok
  caller->>m_todo_board: board.add(#quot;c#quot;)
  m_todo_board->>s_tasks: write
  m_todo_board->>s_next_no: read, write
  m_todo_board->>x_out: line
  m_todo_board-->>caller: 3
```

## `todo.tests.scenarios.empty_board_says_so`

```mermaid
sequenceDiagram
  actor caller as empty_board_says_so
  participant m_todo_board as todo.board
  participant s_tasks as tasks
  participant x_out as out
  caller->>m_todo_board: board.list()
  m_todo_board->>s_tasks: read
  m_todo_board->>x_out: line
  m_todo_board-->>caller: ok
  caller->>m_todo_board: board.stats()
  m_todo_board->>s_tasks: read
  m_todo_board->>x_out: line
  m_todo_board-->>caller: ok
  Note over caller: assert out.text == #quot;nothing to do#92;n0 open, 0 done#92;n#quot;
```

## `todo.tests.scenarios.list_ranks_urgent_first`

```mermaid
sequenceDiagram
  actor caller as list_ranks_urgent_first
  participant m_todo_board as todo.board
  participant s_tasks as tasks
  participant s_next_no as next_no
  participant x_out as out
  Note over s_tasks,s_next_no: given three_open_one_done
  caller->>m_todo_board: board.list()
  m_todo_board->>s_tasks: read
  m_todo_board->>x_out: line
  m_todo_board-->>caller: ok
  Note over caller: assert out.text == #quot;3 [ ] urgent !high #35;work#92;n2 [ ] plain#92;n1 [ ] low one !low#92;n#quot;
```

## `todo.tests.scenarios.list_all_and_by_tag`

```mermaid
sequenceDiagram
  actor caller as list_all_and_by_tag
  participant m_todo_board as todo.board
  participant s_tasks as tasks
  participant s_next_no as next_no
  participant x_out as out
  Note over s_tasks,s_next_no: given three_open_one_done
  caller->>m_todo_board: board.list(all: true, tag: Some(#quot;work#quot;))
  m_todo_board->>s_tasks: read
  m_todo_board->>x_out: line
  m_todo_board-->>caller: ok
  Note over caller: assert out.text == #quot;3 [ ] urgent !high #35;work#92;n4 [x] finished #35;home #35;work#92;n#quot;
```

## `todo.tests.scenarios.done_and_reopen_move_once`

```mermaid
sequenceDiagram
  actor caller as done_and_reopen_move_once
  participant m_todo_board as todo.board
  participant s_tasks as tasks
  participant s_next_no as next_no
  participant x_out as out
  Note over s_tasks,s_next_no: given three_open_one_done
  caller->>m_todo_board: board.done(2)
  m_todo_board->>s_tasks: read, write
  m_todo_board->>x_out: line
  m_todo_board-->>caller: ok
  caller->>m_todo_board: board.done(2)
  m_todo_board->>s_tasks: read
  m_todo_board--xcaller: fail AlreadyDone
  caller->>m_todo_board: board.reopen(2)
  m_todo_board->>s_tasks: read, write
  m_todo_board->>x_out: line
  m_todo_board-->>caller: ok
  caller->>m_todo_board: board.reopen(2)
  m_todo_board->>s_tasks: read
  m_todo_board--xcaller: fail NotDone
  Note over caller: assert out.text == #quot;2 [x] plain#92;n2 [ ] plain#92;n#quot;
```

## `todo.tests.scenarios.rename_keeps_the_rest`

```mermaid
sequenceDiagram
  actor caller as rename_keeps_the_rest
  participant m_todo_board as todo.board
  participant s_tasks as tasks
  participant s_next_no as next_no
  participant x_out as out
  Note over s_tasks,s_next_no: given three_open_one_done
  caller->>m_todo_board: board.rename(3, title: #quot;urgent, really#quot;)
  m_todo_board->>s_tasks: read, write
  m_todo_board->>x_out: line
  m_todo_board-->>caller: ok
  Note over caller: assert board.tasks[3] == Task(3, title: #quot;urgent, really#quot;, priority: High, tags: Set(#quot;work#quot;))
```

## `todo.tests.scenarios.clear_removes_only_done`

```mermaid
sequenceDiagram
  actor caller as clear_removes_only_done
  participant m_todo_board as todo.board
  participant s_tasks as tasks
  participant s_next_no as next_no
  participant x_out as out
  Note over s_tasks,s_next_no: given three_open_one_done
  caller->>m_todo_board: board.clear()
  m_todo_board->>s_tasks: read, write
  m_todo_board->>x_out: line
  m_todo_board-->>caller: 1
  caller->>m_todo_board: board.stats()
  m_todo_board->>s_tasks: read
  m_todo_board->>x_out: line
  m_todo_board-->>caller: ok
  Note over caller: assert out.text == #quot;cleared 1#92;n3 open, 0 done#92;n#quot;
  Note over caller: assert not (4 in board.tasks)
```

## `todo.tests.scenarios.unknown_numbers_are_refused`

```mermaid
sequenceDiagram
  actor caller as unknown_numbers_are_refused
  participant m_todo_board as todo.board
  participant s_tasks as tasks
  participant s_next_no as next_no
  participant x_out as out
  Note over caller: for any no: TaskNo
  Note over s_tasks,s_next_no: given three_open_one_done
  opt no #lt;= 4
    Note over caller: return
  end
  caller->>m_todo_board: board.done(no)
  m_todo_board->>s_tasks: read
  m_todo_board--xcaller: fail NoTask
  caller->>m_todo_board: board.reopen(no)
  m_todo_board->>s_tasks: read
  m_todo_board--xcaller: fail NoTask
  caller->>m_todo_board: board.rename(no, title: #quot;x#quot;)
  m_todo_board->>s_tasks: read
  m_todo_board--xcaller: fail NoTask
  caller->>m_todo_board: board.remove(no)
  m_todo_board->>s_tasks: read
  m_todo_board--xcaller: fail NoTask
  Note over caller: assert out.text == #quot;#quot;
```

## Limits

- The arrows to state and `out` are everything the called rule can touch, not what this call did touch: no rule is executed.
- A refused call shows the reads of the rule's guards only.
- Calls a rule makes to functions are not drawn; see the call graph.
- A note longer than 96 characters is cut and marked with `…`.
