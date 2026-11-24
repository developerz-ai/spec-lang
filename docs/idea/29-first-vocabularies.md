# First vocabularies

> **Design.** Nothing here runs yet. The exact kinds and notes of the first five vocabularies, what each expands to, its default, what it derives and what it leaves outside.

**Rule: a word earns its place only if it expands to core declarations, has one default, and comes with a law a backend can be tested against.** The machinery is in [09-vocabularies](09-vocabularies.md). The order is the corpus ranking in [16-corpus](16-corpus.md): `web`, `errors`, `jobs`, then `game` and `storage` from the roadmap. Every module below parses. Nothing else about them is checked.

## `web`

| Word, and where | Expands to | Default | Law |
|---|---|---|---|
| `entity Name(fields)` in a module | a record `type`, `state Name: Map[Id[Name], Name]`, owned by the module | not readable from outside until `@access` says so | what is read back equals what was written; no path reads it without `@access` |
| `@access(owner -> all, admin -> read)` on an entity | the same Bool on every derived read and write | absent: denied | every surface evaluates the same predicate |
| `@tenant(field)` on an entity | that field added to every filter and every insert | none; cross-tenant reads need a listed declassification ([05](05-effects-and-trust.md)) | no reply carries a row of another tenant |
| `@unique(fields)` on an entity | a law, and an insert that fails `Taken` | none | no two rows agree on the fields, in any interleaving |
| `@append` on an entity | no rule may assign into or remove a row | rows are mutable | a row once written never changes |
| `@cascade` on an `Id[X]` field | removing the `X` removes these rows in the same commit | removing a referenced row fails `InUse` | no dangling id after any commit |
| `@index(fields)` on an entity | nothing: meaning is unchanged | derived from the filters and sorts of the module's queries | none; it is a cost pin, shown in `view --as policy` |
| `query name(params) -> T = expr` in a module | a `fn` with `@uses(X.read)` over committed state | private; `pub` plus `@access` to expose | it never sees half a commit |
| `@live` on a query | result type `Live[T]`; each commit that changes the answer is pushed | not live | once changes stop, the client's value equals the query; `@access` is re-evaluated per subscriber |
| `@page(size, by: keyset)` on a query or rule returning a `List` | a cursor parameter and a page type | every exposed list is paged at 50 by keyset | concatenated pages equal the whole answer when it is stable |
| `page Name(params) = view` with `@at(path)` | a pure `fn` from its parameters to a view tree, plus a route. A parameter's default is the query that feeds it | none; `@access` is required | every failing or pending parameter has a rendered state |
| `@reveals(field)` on a rule | lets one `@secret` field leave in that rule's reply | a `@secret` in a reply is a type error | the field appears in no other reply, log or task journal |
| surfaces: `http:`, `tools:`, `pages:` lists on `app` | which exposed rules and queries each door offers | a `pub` rule listed nowhere is callable only from other modules | a rule behaves the same through every door |

Derived: schema and migrations, parsing at the edge, routes, the typed client, the OpenAPI document, the agent tool schema, loading, empty and error states, cache invalidation.

```slg
use web;

@tenant(owner)
@access(owner -> all)
entity Note(id: Id[Note], owner: Id[User], @pii text: Text[1..=500], at: Instant);

@access(by in User)
@uses(Note.write, clock.read)
pub rule write(by: Id[User], text: Text[1..=500]) -> Id[Note] = {
  let id = Id.fresh();
  Note.insert(Note(id, owner: by, text: text, at: clock.instant()));
  return id;
}

@access(by in User)
@live
@page(20)
pub query mine(by: Id[User]) -> List[Note] = Note.filter(it.owner == by).sort(it.at).reversed;

@at("/notes")
@access(me in User)
page Notes(me: Id[User], notes: Live[List[Note]] = mine(me)) = column(
  form(write, fields: (text)),
  list(notes, empty: text("No notes yet"), row: text(it.text)),
);

@given(User -> [ana, bob])
test only_mine = {
  let id = write(ana, text: "hi");
  assert mine(ana).map(it.id) == [id];
  assert mine(bob) == [];
}

@target(ultimate)
app Site(rules: [write], http: [write, mine], tools: [mine], pages: [Notes]);
```

**Outside `web`:** visual design and motion; the view tree names parts (`column`, `form`, `list`) and a target's design system draws them. Sign-in: `User` and `by` come from an auth vocabulary. Search, file storage and mail are separate vocabularies.

## `errors`

| Word, and where | Expands to | Default | Law |
|---|---|---|---|
| `error Name(data) = "cause {data}"` in a module | `type Name(data)`, plus an entry in the module's error table. The text is the cause, with the data interpolated | a name after `fail` with no declaration is an error with no data, cause equal to its name | every `fail` reachable from an exposed rule is in that rule's computed error set |
| `@status(n)` on an error | the wire status | by where it is raised: denied `@access` and `Missing` 404, a failed `@guard` 409, a refused parameter type 422, anything unlisted 500 | one error has one status on every surface |
| `@fix("...")` on an error | the instruction shown to the caller; may interpolate the data | absent on an exposed error: allowed in `sketch`, `E-OPEN` in `release` | the fix names something that exists: a rule, a parameter, a command |
| `@retryable` on an error | callers and `@retry` may try again unchanged | not retryable | a retryable error left the world unchanged |

Derived: a stable code from module and name (`seats.Taken`), which a rename must alias and never reuse; the wire shape; the union type each client call can fail with; the error reference. Removing an error or changing its status is a breaking guarantee diff.

```slg
use errors;

state held: Map[Int, Time];

@status(409)
@fix("wait until {until}, or hold another seat")
@retryable
error Taken(seat: Int, until: Time) = "seat {seat} is held until {until}";

@status(422)
@fix("pass a seat between 1 and 200")
error NoSuchSeat(seat: Int) = "there is no seat {seat}";

@uses(held.write, clock.read)
@guard(1 <= n <= 200 else NoSuchSeat(n), (held[n] else 0s) <= now() else Taken(n, until: held[n]))
rule hold(n: Int) -> Time = {
  held[n] = now() + 10min;
  return held[n];
}

@given(clock.at(0s))
test second_hold_explains_itself = {
  hold(7) -> 10min;
  hold(7) -> fail Taken(7, until: 10min);
  hold(900) -> fail NoSuchSeat;
  assert Taken.status == 409 and Taken.code == "seats.Taken";
}
```

**Outside `errors`:** the wording. Cause and fix text are written by the author and are most of an error's tokens (139 of them in the measured webhooks module, [12](12-frameworks.md)). Translation is a separate vocabulary keyed on the code.

## `jobs`

Core already has `task`, `event`, `@on(every(d))`, `@retry(n, on:, exhausted:)` and `@idempotent`. This adds schedules, retention, limits, and what a test needs to see.

| Word, and where | Expands to | Default | Law |
|---|---|---|---|
| `daily(at: 9h, zone: z)`, `weekly(...)`, `cron("...")` inside `@on` | a trigger with a due time | `every(d)` counts from start-up | one run per due time, even after downtime: late, never doubled |
| `run.attempt`, `run.due`, `run.key` in a task body | read-only facts of this run | `attempt` is 1 on the first try | `attempt` rises by one per `@retry` and is the same after a crash within an attempt |
| `@retain(d, by: field)` on an entity or a `Map` state | a private `rule purge(n)` and an `@on(every(...)) task` that calls it until nothing is due | batches of 500; period `d / 10`, clamped to 1h..=1d | nothing younger than `d` is removed; everything older goes within one period |
| `@queue(name)`, `@concurrency(n, per: key)`, `@rate(n, per: d)` on a task | the worker pool; a run over a limit waits | queue `default`, unlimited | never more than `n` at once per key; a waiting run is never dropped |
| `tasks.drain()` in a test | runs every started task and delivered event to rest, moving the clock across backoff waits. It does not fire schedules; `clock.advance` does | a test that ends with started tasks still pending fails | the order is start order, so a test replays |

Derived: the queue rows, the step journal, the dead-letter list, operator commands to retry or cancel a run, and the crash test at every step boundary.

```slg
use jobs;

type Line(id: Id[Line], text: Text, at: Time);

@retain(90d, by: at)
state lines: Map[Id[Line], Line];
state given_up: List[Time];

rule give_up() = { given_up.push(now()); }

@on(daily(at: 3h, zone: "UTC"))
@queue(export)
@concurrency(1)
@retry(3, backoff: 1min, on: Unreachable, exhausted: give_up)
@idempotent(run.due)
@uses(http.call)
task export() = {
  let batch = lines.values.sort(it.at).take(100);
  let reply = http.call(url"https://archive.example.com/lines", body: batch, attempt: run.attempt) else fail Unreachable;
  if reply.status >= 500 { fail Unreachable; }
}

@given(clock.at(0s), http.reply(503))
test three_tries_then_gives_up = {
  export.start();
  tasks.drain();
  assert http.calls.map(it.attempt) == [1, 2, 3];
  assert given_up.size == 1 and tasks.dead.size == 1;
}

@given(clock.at(0s), lines -> Map(Id(1) -> Line(Id(1), text: "old", at: 0s)))
test old_lines_go = {
  clock.advance(91d);
  tasks.drain();
  assert lines.size == 0;
}
```

**Outside `jobs`:** exactly-once to a third party, which stays `assumed`. The queue technology, which only `app` picks. Compensation, since undoing a committed rule is logic the author writes.

## `game`

| Word, and where | Expands to | Default | Law |
|---|---|---|---|
| `component Name(fields)` in a module | a record `type` and `state Name: Map[Id[Entity], Name]` | saved, not replicated | save then load gives the same world |
| `@replicated`, `@owner_only`, `@predicted` on a component or field | who is sent the value; whether the client may guess ahead | server only | a client never receives a field it may not see |
| `system name(a: mut A, b: B, dt: Time)` in a module | a `rule` run once for every entity that has all listed components, in entity id order. `mut` marks the ones it writes | none: it needs `@on(tick(...))` | the result does not depend on storage order |
| `tick(hz)` inside `@on` | a fixed step; `dt` is exactly `1 / hz`; `world.tick` counts | none | a slow frame runs more steps, never a longer one |
| `@after(x)`, `@before(x)` on a system | an order between systems in one tick | a system that writes a component runs before those that read it; a tie is `E-OPEN` | the order is total and the same on every machine |
| a `pub rule` in a `use game` module | a command: queued, then applied at the start of the next tick in (tick, player, sequence) order | the authority decides; `@access` is checked there | peers that apply the same commands reach the same state |
| `world.get`, `world.set`, `world.spawn`, `world.each`; in a test `world.seed(n)`, `world.run(ticks)`, `world.hash` | reads and writes of component state; a seeded world, stepped, then one number for all of it | | same seed and commands give the same hash |

Inside a module that `use`s `game`, `clock.read`, unseeded `random` and `Float` are type errors, so determinism is checked where the code is written. Derived: save and load, the command codec, replication messages, the replay log, the tick schedule.

```slg
use game;

component Pos(x: Fixed[m]);
component Vel(v: Fixed[m/s]);
@owner_only component Owner(player: Id[Player]);

@on(tick(30Hz))
@alloc(none)
system move(pos: mut Pos, vel: Vel, dt: Time) = { pos.x = pos.x + vel.v * dt; }

@access(world.get(Owner, id: unit).player == by)
pub rule steer(by: Id[Player], unit: Id[Entity], v: Fixed[m/s]) = {
  world.set(Vel, id: unit, v: v);
}

@given(world.seed(1), world.with(u1, Pos(0m), Vel(0[m/s]), Owner(ana)))
test a_command_applies_on_the_next_tick = {
  steer(ana, unit: u1, v: 3[m/s]);
  world.run(30);
  world.get(Pos, id: u1).x -> 3m;
  steer(bob, unit: u1, v: 9[m/s]) -> fail NotFound;
}

law same_seed_same_world(seed: Int, t: Trace[steer | move]) =
  world.replay(seed, t.steps.map(it.event)).hash == t.states.last.hash;
```

**Outside `game`:** rendering, audio, input devices, assets and animation, which are engine modules reached through `app`. The netcode model (lockstep, rollback, server authority), chosen in `app`. Physics, which is its own vocabulary with its own laws. See [13](13-games.md).

## `storage`

| Word, and where | Expands to | Default | Law |
|---|---|---|---|
| `layout Name(fields) { laws }` in a module | a record `type` plus `encode` and `decode` between it and exactly `@size` bytes | no implicit padding: every byte is a named field | `decode(encode(x)) == x`, and `encode` fills the size exactly |
| `@size(n)`, `@endian(little)` on a layout; `@at(offset)` on a field; field types `Bytes[n]`, `Array[T, n]`, `Rest` | the byte picture | little endian, fields in order | fields do not overlap and fit |
| `Log[T]` as a state type | an append-only sequence with `append`, `sync`, and the reads `size`, `durable`, `disk` | an append is not durable until `sync` | after `sync` returns, `durable == size`; the durable prefix survives a crash |
| `Pages[P]` as a state type | fixed-size slots with `read(id)`, `write(id, p)`, `sync()` | a write is not durable until `sync` | a torn write is seen as a failed check, never as a valid page |
| `crash` in a `Trace[...]` | an event that drops everything not yet durable | absent from traces unless listed | |
| `@durable(expr)` on a rule | a `@post` over the durable part of the state | none | it holds after any crash that follows the reply |

Derived: reader, writer and a consistency checker for each layout, recovery tests that crash between every two writes, and upgrade code between two versions of a layout ([22](22-lifecycle.md)).

```slg
use storage;

@size(64B)
@endian(little)
layout Slot(crc: u32, lsn: u64, len: u16, _pad: Bytes[2], data: Bytes[48]) {
  law codec(s: Slot) = decode(encode(s)) == s;
  law honest(s: Slot) = s.crc == crc32(s.data.take(s.len));
}

type Put(key: u64, value: Bytes[48]);
state log: Log[Put];
state table: Map[u64, Bytes[48]];

@uses(log.write, table.write)
pub rule put(key: u64, value: Bytes[48]) = {
  log.append(Put(key, value: value));
  table[key] = value;
}

@uses(log.sync)
@fails(DiskError)
@durable(log.durable == log.size)
pub rule flush() = { try log.sync(); }

fn replay(puts: List[Put]) -> Map[u64, Bytes[48]] =
  puts.fold(Map(), (m, p) => m.with(p.key -> p.value));

law flushed_survives(t: Trace[put | flush | crash]) =
  t.states.all(s => replay(s.log.disk) == replay(s.log.take(s.log.durable)));

@given(log -> [])
test unflushed_is_lost = {
  put(1, value: b"a");
  flush();
  put(2, value: b"b");
  crash();
  assert replay(log.disk).keys == [1];
}
```

**Outside `storage`:** the buffer manager's eviction policy, latching and lock-free structures, which are `machine` bodies under these types. Query planning and transactions across pages, which are logic the author writes as rules and laws. See [14](14-systems.md).

## Limits

- **None of these is implemented or tested.** A wrong expansion gives a wrong meaning every test agrees with.
- **Defaults are guesses.** Page size 50, retention period `d / 10`, status by raise site: each is overridable, and whether it is right often enough is unmeasured.
- **`web` leans on the backend for atomicity.** `@unique` and a counted `@guard` are one line here and a constraint, a lock or a refusal on a real database ([12](12-frameworks.md)).
- **`@live` with per-subscriber `@access`** needs a cheap predicate. One that reads another entity per row per subscriber is a cost the vocabulary can report and not remove.
- **`game` order against speed.** Entity id order makes results reproducible and may forbid the storage layouts that make an entity store fast.
- **A library name never both changes a place and returns.** `log.append` therefore returns nothing, and a rule that needs the new position reads `log.size` after it. The B-tree example in [24](24-examples.md) still writes `let lsn = wal.append(...)`, which conflicts with that card rule.
- **`@reveals` is the only declassification here.** Wider ones belong to [05](05-effects-and-trust.md) and are not specified.
