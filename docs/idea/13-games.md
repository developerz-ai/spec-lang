# Games

> **Design.** Nothing here runs yet. This doc covers deterministic simulation logic: how it would lower onto an in-house C++ sim today and onto anygame later, and what stays hand-written.

**Rule: the simulation is rules over a tick, the engine is a backend, and determinism is a claim the compiler checks instead of a lint.** Rendering, audio and the hot inner loops stay at machine altitude or in the engine. No feature has been measured here, so this doc claims no ratio.

## The case in hand

An in-house C++ simulation game, described by generic shape only. Sizes are rounded hard and nobody outside can check them.

| Fact | Shape |
|---|---|
| Source | over 100k lines |
| Split | roughly half simulation side (rules, commands, save, net), half presentation (UI, renderer, audio) |
| Tests | the same order of size |
| Model | fixed-point, fixed tick, lockstep over an ECS library |

Its hard rules are the interesting part. Most are common to deterministic games, and each is held today by a script or a debug check that runs after the code is written.

| Rule held today by a lint or a debug check | In spec-lang | Kind of evidence |
|---|---|---|
| No floats, clocks or unordered iteration in the sim | A sim rule's `@uses` cannot include `clock.read` or `random.read`; `Fixed` is the only fractional number in scope; collections iterate in a defined order | type error |
| All mutation goes through one command type | A `pub rule` is the command. Nothing else can write `state` | by construction |
| Every balance change goes through one ledger | `cash` is written by one private rule. A `law` ties balance to entries | proven or bounded |
| Saved data keeps its format, with a round-trip test | Types carry versions; migrations and the round-trip test are derived ([`22-lifecycle.md`](22-lifecycle.md)) | derived test |
| Sim never includes render or UI headers | The sim module does not `use` them, and effects would show it if it tried | type error |

Moving a rule from "a script greps for it" to "it does not type-check" is the main gain for a game. The token saving is secondary and unmeasured.

## What the logic looks like

A sketch in the card's syntax with the `game` vocabulary. It parses. The setting (batches moving along workshop stations) is invented and is not the game's design.

```slg
use game;

type Cash = Int;
type Entry(owner: Id[Owner], amount: Cash, why: Text, tick: Int);
state cash: Map[Id[Owner], Cash];
state ledger: List[Entry];

component Station(owner: Id[Owner], work: Fixed[s], next: Option[Id[Station]], closed: Bool = false);
component Batch(owner: Id[Owner], on: Id[Station], done: Fixed[s], pace: Fixed, units: Int = 0);
event finished(batch: Id[Batch]);

@why("every cash change is an entry, so the books always explain the balance")
rule book(owner: Id[Owner], amount: Cash, why: Text) = {
  cash[owner] = cash[owner] + amount;
  ledger.push(Entry(owner, amount: amount, why: why, tick: world.tick));
}

@on(tick(10Hz))
@after(commands)
@alloc(none)
@within(2ms, per: 2_000)
system advance(batch: mut Batch, dt: Time) = {
  let station = world.get(Station, id: batch.on);
  if station.closed or batch.units == 0 { return; }
  let to = batch.done + batch.pace * dt;
  if to < station.work {
    batch.done = to;
    return;
  }
  match station.next {
    Some(n) => {
      batch.on = n;
      batch.done = to - station.work;
    }
    none => {
      batch.done = station.work;
      finished.emit(world.id(batch));
    }
  }
}

@on(finished)
rule settle(batch: Id[Batch]) = {
  let t = world.get(Batch, id: batch);
  book(t.owner, amount: t.units * 120, why: "finished batch");
  world.set(Batch, id: batch, units: 0);
}

@access(world.get(Station, id: from).owner == by)
@guard(cash[by] >= work.whole * 50 else Broke)
@guard(not world.get(Station, id: from).next.is(Some(_)) else Occupied)
pub rule extend(by: Id[Owner], from: Id[Station], work: Fixed[s]) -> Id[Station] = {
  let made = Id.fresh();
  world.spawn(made, Station(by, work: work, next: none));
  world.set(Station, id: from, next: Some(made));
  book(by, amount: 0 - work.whole * 50, why: "new station");
  return made;
}

law books_explain_cash(t: Trace[extend | advance | settle], c: Id[Owner]) =
  t.states.all(s => s.cash[c] == s.ledger.filter(it.owner == c).map(it.amount).sum);

@given(world.seed(42), scenario.two_shops)
test same_seed_same_world = {
  world.run(20_000);
  let first = world.hash;
  world.restart();
  world.run(20_000);
  assert world.hash == first;
}
```

What the author did not write: the command variant and its wire codec, save and load for two components, the replay entry, the system order, or any registration. Who may extend a station is one `@access` line, checked on every path a command can arrive by.

## Lowering today: the in-house engine

The shapes below are what a game of this kind would need. The game's "framework" is its own conventions, so the backend is specific to that codebase. That is acceptable: a backend is a contract ([`11-backends-and-parity.md`](11-backends-and-parity.md)), not a product.

| spec-lang | Emitted C++ shape | Hand-written today, then derived |
|---|---|---|
| `component` | a plain struct, a registry entry, a save codec entry | the save and validate code that must be edited in the same change |
| `pub rule` | a case of the command variant, its codec, a validator, a branch in the one execute function | codec and replay plumbing |
| `system` with `@on(tick)`, `@after` | a function over an ECS view, called at a fixed place in the tick | the tick order, now computed from declared reads and writes |
| `event` and `@on(event)` | an entry in the existing event queue, drained at a fixed point | queue wiring |
| `Fixed[unit]` | the game's existing fixed-point type; units erased | unit mistakes |
| `law` | a debug invariant compiled into the checked build, plus a property test | hand-written invariants |
| `test` | a unit test, and for scenarios a headless run | fixtures |

### Parity is unusually cheap here

A deterministic game can run headless from a seed and print a hash of the whole world; this one does. That is a ready-made parity check: the interpreter and the generated C++ run the same seed and the same commands for N ticks and must print the same hash. A web app has no equivalent single number.

For that to work the language has to pin down three things that most languages leave open:

| Must be defined | Why |
|---|---|
| Iteration order of every collection and every ECS query | the hash depends on it, and ECS libraries do not promise one |
| Overflow of `Fixed` | exact in the interpreter and wrapping in C++ would diverge on the first large product |
| The order of systems and of events within a tick | lockstep peers must agree |

## The cycle matters more than the size

Game code barely shrinks ([16](16-corpus.md)). The larger cost is waiting: a full native build and a run stand between a wrong rule and finding out. Many of those mistakes are in the logic, and the logic does not need the engine.

| Step | Full native game | Logic in spec-lang (design, unmeasured) |
|---|---|---|
| change a rule | edit, compile, launch, reach the situation | edit; the interpreter reruns only affected tests, with no build |
| several agents | share the compiler | each runs logic at once |
| a rare interaction | play, or sweep seeds until it appears | a law over traces, shrunk to the moves that cause it |
| then | | the same tests run on the generated code |

Two parts stay slow. Rendering, geometry, art and feel need the real build and eyes on a screen. Long sweeps need compiled speed, because an interpreter is far slower per tick: logic first in the interpreter, sweeps on the generated build. A project that already has fast headless unit tests gains less; the comparison is against its compile and queue.

## Lowering later: anygame

anygame is a public engine design with no engine code yet. Its design lists twelve declared primitives. The ones spec-lang would target:

| spec-lang | anygame primitive (planned) |
|---|---|
| `component` | `component`, schema-described |
| `system` with declared reads and writes | `system` |
| `event` | `event` |
| `pub rule` on a sim | `command`, validated by the authority |
| `@access` | `policy` |
| `task` | `job` |
| versioned types | `migration` |
| `test` | `test`, headless with bots |

Two of its ideas line up with spec-lang directly. Its systems declare reads and writes, which is `@uses`. Its determinism is opt-in per world, which is the absence of `clock.read` and `random.read`.

What must exist in anygame before a backend is worth starting: the reflection and schema phase, the ECS, the command-line tool that describes a project, and a headless test runner.

One request while it is still on paper. Its plan derives the schema from annotated C++ with a clang-based tool. If the versioned schema were also a stable *input*, spec-lang could emit the schema and the structs together and never depend on attribute syntax that the design marks as not final.

The promise worth testing is that the module above compiles to both engines unchanged. Until a second engine backend passes parity, that is a hope.

## What stays below

| Stays hand-written or in the engine | Why |
|---|---|
| Renderer, mesh generation, shaders, audio | no logic to state above the mechanism; these are `machine` or `foreign` |
| The fixed-point type itself, ECS storage and layout | the engine's job, used through the backend |
| Pathfinding and other inner loops | may be written in spec-lang at `machine` altitude with `@within` and `@alloc(none)`, or kept `foreign` when the tuned version wins |
| UI layout | a view problem, see [`12-frameworks.md`](12-frameworks.md) on why views are not unified |

A plausible first scope is the rules of play: perhaps a third of the tree. That fraction is an estimate from directory sizes, not from converting anything.

## Limits

- **Nothing is measured.** No feature of the sim was rewritten. Every "removed" above is a class of code, not a count.
- **Balance needs speed.** Tuning is judged over many seeds and long runs. An interpreter will not do that in useful time. Sweeps run on generated C++, so the fast loop of idea, logic, test only covers logic tests.
- **Generated code against hand-tuned code.** A `@within` budget reports a miss on weak hardware, and does not fix it.
- **Defined order has a cost.** Pinning iteration order may forbid the storage layouts that make an ECS fast. This tension is not resolved.
- **A large test suite** holds the current game together. Lifting logic is only safe when those tests can run against generated code, which means the generated C++ must keep the names and shapes the tests use.
- **A bespoke backend is real work** for one game. It pays only if the game keeps growing or the same backend shape serves the next one.
