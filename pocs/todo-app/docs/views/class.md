# Class diagram

> **Generated.** Projection `diagram --kind class` of the program's IR, printed by the `view` crate. Do not edit: change the source and regenerate. Same source, same bytes.

**8 classes, 6 relations.**

```mermaid
classDiagram
  class todo_laws_Change["Change"] {
    <<rule set>>
    add
    done
    reopen
    rename
    remove
    clear
  }
  class todo_model_Title["Title"] {
    <<type>>
    Text
  }
  class todo_model_Tag["Tag"] {
    <<type>>
    Text
  }
  class todo_model_TaskNo["TaskNo"] {
    <<type>>
    Int
  }
  class todo_model_Status["Status"] {
    <<enumeration>>
    Open
    Done
  }
  class todo_model_Priority["Priority"] {
    <<enumeration>>
    Low
    Normal
    High
  }
  class todo_model_Task["Task"] {
    <<record>>
    no: TaskNo
    title: Title
    status: Status = Open
    priority: Priority = Normal
    tags: Set[Tag] = Set#40;#41;
  }
  class todo_board["todo.board"] {
    <<module>>
    next_no: TaskNo = 1
    +add(title: Title, priority: Priority = Normal, tags: Set[Tag] = Set()) TaskNo
    +list(all: Bool = false, tag: Option[Tag] = none)
    +done(no: TaskNo)
    +reopen(no: TaskNo)
    +rename(no: TaskNo, title: Title)
    +remove(no: TaskNo)
    +clear() Int
    +stats()
  }
  note for todo_model_Title "{ is_title(self) }"
  note for todo_model_Tag "{ is_tag(self) }"
  note for todo_model_TaskNo "{ self #gt;= 1 }"
  todo_model_Task "1" *-- "1" todo_model_TaskNo : no
  todo_model_Task "1" *-- "1" todo_model_Title : title
  todo_model_Task "1" *-- "1" todo_model_Status : status
  todo_model_Task "1" *-- "1" todo_model_Priority : priority
  todo_model_Task "1" *-- "0..*" todo_model_Tag : tags
  todo_board "1" o-- "0..*" todo_model_Task : tasks [TaskNo]
```

| Class | Kind | Declaration | Constraints |
|---|---|---|---|
| `Change` | rule set | `todo.laws.Change` | - |
| `Title` | type | `todo.model.Title` | `is_title(self)` |
| `Tag` | type | `todo.model.Tag` | `is_tag(self)` |
| `TaskNo` | type | `todo.model.TaskNo` | `self >= 1` |
| `Status` | enumeration | `todo.model.Status` | - |
| `Priority` | enumeration | `todo.model.Priority` | - |
| `Task` | record | `todo.model.Task` | - |
| `todo.board` | module | `todo.board` | - |

A note `{ claim }` is a constraint every value of the type satisfies. A filled diamond is a field (the record holds the value); a hollow diamond is a module's `state`, labelled with the map key in brackets.

## Limits

- Functions are not drawn; a claim that calls one shows the call as text only.
- A type named only through `Trace[...]` or a rule set has no relation line.
- Multiplicity is read off the type (`Option`, `List`, `Set`, `Map`), not off claims that bound a size.
