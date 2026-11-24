# Pseudocode

> **Generated.** Projection `pseudo` of the program's IR, printed by the `view` crate. Do not edit: change the source and regenerate. Same source, same bytes.

Every function and rule as textbook pseudocode: `←` assigns, `=` compares, `require p else fail E` is a guard checked before the body, `emit` asks for an outside effect after commit.

## `todo.board`

```text
module todo.board

state tasks: Map[TaskNo, Task], initially empty

state next_no: TaskNo, initially 1

function has_tag(t: Task, tag: Option[Tag]) → Bool
  match tag
    case none: return true
    case Some(x): return x in t.tags
  end
end

rule add(title: Title, priority: Priority = Normal, tags: Set[Tag] = Set()) → TaskNo
  because: a number is never reused, so an old note that says 'task 3' never names a new task
  on command "add"
  uses tasks.write, next_no.write, out.write
  let no ← next_no
  tasks[no] ← Task(no, title, priority: priority, tags: tags)
  next_no ← no + 1
  emit out.line("added {no}")
  return no
end

rule list(all: Bool = false, tag: Option[Tag] = none)
  because: open work first; finished work only when asked for
  on command "list"
  uses tasks.read, out.write
  let shown ← tasks.values.filter(t ↦ (all or t.status = Open) and has_tag(t, tag)).sort(t ↦ rank(t))
  if shown.size = 0 then
    emit out.line("nothing to do")
    return
  end
  for each t in shown do
    emit out.line(line(t))
  end
end

rule done(no: TaskNo)
  because: finishing is one step, and only open work can finish
  on command "done"
  uses tasks.write, out.write
  require no in tasks else fail NoTask
  require tasks[no].status = Open else fail AlreadyDone
  tasks[no].status ← Done
  emit out.line(line(tasks[no]))
end

rule reopen(no: TaskNo)
  because: a mistaken done is undone, not re-added under a new number
  on command "reopen"
  uses tasks.write, out.write
  require no in tasks else fail NoTask
  require tasks[no].status = Done else fail NotDone
  tasks[no].status ← Open
  emit out.line(line(tasks[no]))
end

rule rename(no: TaskNo, title: Title)
  because: a title changes without losing its number, status or tags
  on command "rename"
  uses tasks.write, out.write
  require no in tasks else fail NoTask
  tasks[no].title ← title
  emit out.line(line(tasks[no]))
end

rule remove(no: TaskNo)
  because: removing frees nothing: the number stays used
  on command "remove"
  uses tasks.write, out.write
  require no in tasks else fail NoTask
  remove no from tasks
  emit out.line("removed {no}")
end

rule clear() → Int
  because: finished work leaves the list in one step
  on command "clear"
  uses tasks.write, out.write
  let gone ← tasks.values.filter(it ↦ it.status = Done)
  for each t in gone do
    remove t.no from tasks
  end
  emit out.line("cleared {gone.size}")
  return gone.size
end

rule stats()
  because: a count at a glance, without listing
  on command "stats"
  uses tasks.read, out.write
  let open ← tasks.values.filter(it ↦ it.status = Open).size
  emit out.line("{open} open, {tasks.size - open} done")
end
```

## `todo.model`

```text
module todo.model

function is_title(t: Text) → Bool
  because: a title fits on one line of a terminal list
  return t.trim().size ≥ 1 and t.size ≤ 200 and t.bytes.all(it ↦ it ≥ 32 and it ≠ 127) and not t.contains("") and not t.contains(" ") and not t.contains(" ")
end

function is_tag(t: Text) → Bool
  because: a tag is typed in a shell without quotes, and never reads as a flag
  return 1 ≤ t.size ≤ 32 and t.bytes.all(it ↦ 97 ≤ it ≤ 122 or 48 ≤ it ≤ 57 or it = 45) and not t.starts_with("-")
end
```

## `todo.render`

```text
module todo.render

function mark(s: Status) → Text
  match s
    case Open: return " "
    case Done: return "x"
  end
end

function badge(p: Priority) → Text
  match p
    case Low: return " !low"
    case Normal: return ""
    case High: return " !high"
  end
end

function tag_text(tags: Set[Tag]) → Text
  return tags.list.map(t ↦ " #{t}").join("")
end

function urgency(p: Priority) → Int
  match p
    case High: return 0
    case Normal: return 1
    case Low: return 2
  end
end

function rank(t: Task) → (Int, Int)
  because: most urgent first, then oldest first
  return (urgency(t.priority), t.no)
end

function line(t: Task) → Text
  return "{t.no} [{mark(t.status)}] {t.title}{badge(t.priority)}{tag_text(t.tags)}"
end
```

## Limits

- Types, tests, laws and apps are not printed here.
- Core library methods keep their spec-lang names (`filter`, `sort`); they are not expanded into loops.
- A reading aid, never an input: it is not parsed back.
