# Hygiene

> **Design.** Nothing here runs yet. The habits that keep a large program healthy, and which of them the language keeps for you.

**A habit that needs discipline gets forgotten. Each one here is either impossible to break, checked, or derived; the rest are named as still yours.**

## Cleanup, after RAII

C++ ties cleanup to scope, so it cannot be forgotten on any exit. Rust makes the owner explicit. Both ideas are kept, at the altitude where they matter.

| Level | Resource | Released |
|---|---|---|
| `fn`, `rule` | none: values only, and a rule's writes are all or none | nothing to leak |
| `task` | a handle: a connection, a file, a lease | at the end of its block on every exit: return, failure, cancel |
| `task`, crash | the same | the holder is gone, so a lease needs a deadline |
| `machine` | `own`: memory, frames, interrupts off | exactly once, checked; using it after is a type error |
| `app` | handlers | when the app stops, in reverse order |

```slg
@edge(Held -> Free, via: give_back)
@edge(Held -> Free, after: 10min)
flow Lease(at: Box.lease);

@uses(http.call)
task work(box: Id[Box], job: Id[Job]) = {
  let turn = boxes.lease(box) else fail Busy;
  let reply = http.call(url"https://example.com/run", body: job) else fail Unreachable;
  record(job, result: reply.status);
}

law nothing_held_forever(t: Trace[take | give_back], b: Id[Box]) =
  t.within(10min, it.boxes[b].lease == Free);
```

`turn` is given back when the block ends, however it ends. The deadline covers the one case scope cannot: the machine died. Owned handles in a `task` and `lease` are proposals; the card allows `own` in `machine` only.

## The list

| Habit | Kept by | How |
|---|---|---|
| no leaks | scope, `own`, deadlines | above |
| no shared mutable state | module-owned state, immutable values | checked |
| no null, no uninitialised | `Option`, defaults, initial state | impossible |
| no swallowed errors | failures are in the type; `else` is explicit | checked |
| no unchecked input | entering data is untrusted until decoded | checked |
| no secrets in logs | `@secret`, `@pii` follow the value | checked |
| no magic numbers without units | units on literals | checked |
| no dead code, unused name, unreachable arm | reported by the checker | checked |
| no copy of a rule | one declaration, every surface projected from it | derived |
| no orphan rows | deleting something referenced is `E-OPEN` until answered | asked |
| no unbounded growth | `@page`, `@within`, retention in a vocabulary | asked |
| no half-done work | atomic rules; tasks that resume or compensate | derived |
| no double delivery | `@idempotent`, or the report says it can happen | asked |
| no zombie work | every `task` has an end, a deadline or an owner | a law to write |
| no stale documents | views are printed from source | derived |
| no drifting configuration | one lock file, hashes | derived |
| no hidden dependency | `use`, effects, contracts | checked |
| no mystery build | same source and lock, same bytes | derived |
| one format | the formatter | impossible to break |
| small pieces | the slice size to edit one rule is reported | reported |
| clear names | - | **still yours** |
| the right abstraction | - | **still yours** |
| knowing when to delete | unused declarations are listed | **still yours** |
| the right law | - | **still yours** |

## Macro hygiene

A vocabulary cannot capture a name from the module that uses it, or leak one into it. Its expansion names come from the declaration and its role, so two expansions never collide, and it cannot add a second writer to someone's state ([09](09-vocabularies.md)).

## Security hygiene

Deny by default with a caller. Capabilities come from `app` and can only shrink on the way down. Packages run no code when opened, checked or built. A new effect in an upgrade is a guarantee diff that someone approves ([05](05-effects-and-trust.md), [08](08-approval-and-anti-cheating.md)).

## Repository hygiene

Source is the only thing edited. Generated output is ignored by git or frozen by hash. No flags, no shims, no two ways at once: a change replaces the old meaning, and the migration is a typed step ([22](22-lifecycle.md)).

## Limits

- "Checked" means checked of the meaning. Generated code and foreign code keep hygiene only as far as parity and contracts show.
- Scope cannot release what another machine holds. Every such resource needs a deadline, and choosing it is a guess about the world.
- A habit turned into an error can be worked around with `@assume` or `foreign`. Both are listed, and lists can be ignored.
- None of the table's rows has been enforced by a running checker.
