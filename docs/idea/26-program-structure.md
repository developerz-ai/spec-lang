# Program structure

> **Design.** Nothing here runs yet. How to lay out a program, from one file to a system with many targets.

**Structure exists to define logic fast and test it fast.** A module is the smallest piece whose logic can be tested alone. Everything below follows from that.

## Start with one file

No project, no target, no folders. A file is a program:

```slg
type Level = 0..=10;
state level: Level = 0;

@guard(level < 10 else Top)
rule up() -> Level = {
  level = level + 1;
  return level;
}

@given(level -> 9)
test stops_at_the_top = {
  up() -> 10;
  up() -> fail Top;
}
```

`spec-lang test` on this file is the whole loop. A target is added when someone needs software, not before.

## A module is one noun and its life

Cut by what changes together, never by layer. There is no folder for types, services, routes or tests.

| In the same file, in this order | Why |
|---|---|
| `use` | what it needs |
| types, records, sums | the words |
| `state`, `flow` | what changes, and the moves allowed |
| `pub` rules and queries, then private ones | what can happen |
| pure `fn`s | how it is worked out |
| `event`s it emits | what others may hear |
| laws | what is always true |
| tests | beside the logic they check |

A formatter prints that order, so a model never decides it. Declaration order carries no meaning.

## Four rules

1. **A module owns its state.** Only its rules write it. Others call its `pub` rules and queries. So every law about a state can be checked by reading one module.
2. **`use` has no cycles.** If `orders` uses `stock`, `stock` cannot use `orders`. It emits an event instead, and `orders` listens.
3. **Modules name no target.** Only an `app` names backends, places and handlers. The same module runs in the interpreter and on every backend.
4. **The `pub` part is the contract.** Types, effects and errors are stated there. Private parts are inferred and can change freely.

```slg
event reserved(order: Id[Order], item: Id[Item]);

state free: Map[Id[Item], Int];

@uses(free.write)
@guard(item in free else NoItem, free[item] > 0 else SoldOut)
pub rule reserve(order: Id[Order], item: Id[Item]) = {
  free[item] = free[item] - 1;
  reserved.emit(order, item: item);
}

law never_negative(t: Trace[reserve], i: Id[Item]) =
  t.states.all(s => i in s.free implies s.free[i] >= 0);
```

## When to split

| Signal | Do |
|---|---|
| the context slice to edit one rule passes about 2,500 tokens | split by a second noun or a second life |
| two parts never share a state or a law | split |
| a law needs state from two modules | put the law in a module that uses both; if it is about one invariant, they were one module |
| two modules always change in the same commit | merge |
| a `fn` is used by three modules | move it to a module of its own |
| a file is long but is one life | leave it |

2,500 is a starting value, to be tuned against measured editing errors. Length in lines is not a signal.

## From one file to a system

```text
1 one file        level.slg
2 an app          app.slg  orders.slg  stock.slg  payments.slg
3 domains         app.slg
                  sales/orders.slg   sales/invoices.slg
                  stock/items.slg    stock/counts.slg
                  vocab/house.slg    foreign/bank/contract.slg
                  spec-lang.lock
4 many apps       apps/web.slg  apps/admin.slg  apps/mobile.slg  apps/worker.slg
                  domains/...    one set of modules, several apps
```

| Folder | Holds | Rule |
|---|---|---|
| domain folders | logic | the only place rules live |
| `vocab/` | kinds and notes this program adds | few; a vocabulary is a language extension ([09](09-vocabularies.md)) |
| `foreign/` | contracts for code outside | every entry names its evidence |
| `apps/` or `app.slg` | targets, places, handlers, what is exposed | no logic |
| `spec-lang.lock` | vocabulary, backend and plan pins | committed |
| generated output | per backend, in the place its framework expects | never edited |

Moving from 1 to 4 renames paths and changes no logic. Declarations keep their identity, so tests, evidence and history follow ([07](07-agent-loop.md)).

## Directories to copy

A path is a name: `sales/orders.slg` is `sales.orders`. So the tree is the table of contents, and a model can guess where a thing is before it searches.

**SaaS on Ultimate**

```text
shop/
  spec-lang.lock
  apps/       web.slg  admin.slg  worker.slg
  people/     users.slg  orgs.slg  invites.slg  roles.slg
  sales/      orders.slg  carts.slg  refunds.slg
  billing/    plans.slg  seats.slg  invoices.slg
  notify/     mail.slg  webhooks.slg
  screens/    dashboard.slg  orders.slg  settings.slg
  vocab/      house.slg
  foreign/
    payments/ contract.slg  pretend.slg  real.ts  real.rb
    storage/  contract.slg  pretend.slg  real.ts
  out/        generated, ignored by git
```

**Game**

```text
trains/
  apps/       desktop.slg  server.slg  headless.slg
  world/      map.slg  terrain.slg  time.slg
  track/      segments.slg  signals.slg  building.slg
  trains/     movement.slg  orders.slg  breakdowns.slg
  economy/    cash.slg  cargo.slg  companies.slg
  players/    commands.slg  ai.slg
  data/       engines.slg  cargo_types.slg
  foreign/    render/  audio/
```

**Database or kernel**

```text
store/
  apps/       server.slg  embedded.slg
  pages/      layout.slg  cache.slg
  log/        records.slg  recovery.slg
  tx/         locks.slg  snapshots.slg
  query/      plan.slg  run.slg
  machine/    allocator.slg  checksum.slg
  foreign/    disk/  boot/
```

**A module inside an existing app**

```text
existing-app/
  apps/web/...                 hand-written, unchanged
  spec/   webhooks.slg  seats.slg  app.slg
  spec/foreign/  orgs.slg      what the old code offers
  spec-lang.lock
```

**Many products, one repository**

```text
company/
  spec-lang.lock               one set of versions for all
  shared/     money.slg  users.slg  audit.slg
  shop/       apps/  sales/  billing/
  support/    apps/  tickets/  replies/
```

| Rule | Why |
|---|---|
| folders are areas of the business or the machine, two levels deep at most | deeper paths cost tokens and hide things |
| a folder has a reason to exist at two modules | one module needs no folder |
| no `utils`, `common`, `helpers`, `types`, `models`, `services`, `tests` | they name a kind of file and not a subject, and everything drifts there |
| `shared/` only for what three products use | before that, copy or leave it |
| screens are apart from logic | a screen changes for reasons the logic does not |
| one contract folder per outside provider, with its pretend version beside the real ones | [35](35-packages.md) |
| fixed names: `apps` `vocab` `foreign` `out` | a model finds them in any project |
| no barrel or index files | a path already says what to `use` |
| data tables are modules of `const` | checked like anything else |
| nothing generated outside `out/`, except the embedded layout | [32](32-build-layout.md) |

`spec-lang view map` prints the tree with a line per module: its first `@why`, its states, its `pub` names and its size in tokens. That is the first thing an agent reads ([07](07-agent-loop.md)). Nobody maintains it.

Moving a file is an edit to a name: every `use` follows, and identity, tests and evidence stay.

## The app

```slg
use sales.orders;
use stock.items;

@target(ultimate)
app Shop(rules: [orders.place, orders.cancel, items.search]);

@target(go)
@place(worker)
app Pipeline(rules: [orders.ship]);
```

What is listed is exposed, and needs `@access` ([27](27-authorization.md)); what is not listed, such as `reserve` above, is unreachable from outside. Two apps over the same modules share one set of types, so nothing is synchronised by hand ([11](11-backends-and-parity.md)).

## The same shape in each domain

| Program | Modules | `app` picks |
|---|---|---|
| web product | one per noun: orders, invites, billing | framework, database, surfaces |
| game | one per mechanic: movement, combat, economy; no render code | engine, tick, platforms |
| mobile | the same domain modules plus screens | SwiftUI, Compose, sync |
| database | pages, log, transactions, planner | file or device |
| kernel | frames, scheduler, system calls, drivers | chip, memory map |
| infrastructure | one per service and network | cloud, provisioner |
| script | one file | none: it runs in the interpreter |

## Inside an existing app

One module beside hand-written code, in `spec/`. Its neighbours appear as `use`d interfaces lifted from the framework or as `foreign` contracts. It owns its state like any other module; the old code calls its `pub` rules. Replace one declaration at a time ([12](12-frameworks.md), [22](22-lifecycle.md)).

## Tests and worlds

| Kind | Lives |
|---|---|
| table, scenario, property, law about one module | in that module |
| scenario across modules | in the module that `use`s both, usually the one the user acts on |
| a shared starting world | a `test` in the owning module, named by `@given(...)` elsewhere |
| conformance of a backend | with the backend, never in the program |

Checking a module needs only the contracts it uses. Running a test needs the bodies it actually calls and nothing else, and reruns only when one of those changes. The cost follows the logic a test touches, not the size of the program.

## Names

Modules and folders: lower-case nouns. Types, sums, flows, kinds that declare a thing: `PascalCase`. Rules: verbs. Events: past participles. Laws: the claim (`never_negative`). Tests: the expectation (`stops_at_the_top`). Reserved words cannot be names.

## Several agents at once

The declaration is the lock, not the file ([07](07-agent-loop.md)). A good cut still helps: agents that own different modules share only contracts, so a change behind a contract cannot break the other. Agree on `pub` signatures, laws and tests first, leave `?holes`, then fill modules in parallel, with smaller models if that works ([06](06-logic-and-tests.md)).

## Limits

- Module-owned state forbids one update across two modules' states except by a rule calling the other's rule, which joins its commit inside one place. Across places it becomes a `task`, with a moment when one half is done.
- Breaking a cycle with an event changes meaning: the listener runs later and at least once. Sometimes the right answer is one module.
- Cutting by noun is a habit from records. Compilers, planners and simulations cut better by stage or mechanic, and no rule here decides it.
- Nothing has been laid out at stage 3 or 4, so the thresholds are guesses.
