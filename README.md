<h1 align="center">spec-lang</h1>

<p align="center"><strong>Executable pseudocode for AIs: testable at once, compilable into real software.</strong></p>

<p align="center">The AI writes logic and tests. The interpreter runs them at once. The compiler generates the software.</p>

<p align="center"><em>An executable whiteboard: the program is theory that never runs, only gets verified, and compiles into software that does.</em></p>

<p align="center">
<a href="docs/idea/00-thesis.md">Thesis</a> ·
<a href="docs/idea/01-language-card.md">Language card</a> ·
<a href="docs/idea/24-examples.md">Examples</a> ·
<a href="docs/idea/README.md">All design docs</a> ·
<a href="docs/decisions/README.md">Decisions</a>
</p>

## At a glance

One rule from the [todo POC](pocs/todo-app/README.md), and the Ruby that `spec-lang build` generated from it:

<table>
<tr><th>spec-lang: what must happen</th><th>Ruby: generated, never edited</th></tr>
<tr>
<td>

```slg
@why("finishing is one step, and only open work can finish")
@on(cli("done"))
@uses(tasks.write, out.write)
@guard(no in tasks else NoTask,
       tasks[no].status == Open else AlreadyDone)
pub rule done(no: TaskNo) = {
  tasks[no].status = Done;
  out.line(line(tasks[no]));
}

test done_and_reopen_move_once = {
  board.done(2);
  board.done(2) -> fail AlreadyDone;
}
```

</td>
<td>

```ruby
# in module Todo::Board
# rule done (todo/board.slg:44)
# @why finishing is one step, and only open work can finish
def self.done(no)
  Spec.rule do
    Todo::Model::TaskNo.check(no, "argument `no`")
    Spec.guard("NoTask") { tasks.key?(no) }
    Spec.guard("AlreadyDone") { Spec::Maps.at(tasks, no).status == Todo::Model::Status::Open }
    self.tasks = Spec.put(tasks, [Spec.key(no), :status], Todo::Model::Status::Done)
    Spec.effect("cli.out", :line, Todo::Render.line(Spec::Maps.at(tasks, no)))
    nil
  end
end
```

</td>
</tr>
</table>

The left side is all anyone writes. `spec-lang test` checks it in memory, with no database, disk or network. The same source also builds to Rust, TypeScript and C++, and the command line, the JSON store, file locking and the tests around this rule are derived, not written. The Ruby is one method of its module, de-indented ([full file](pocs/todo-app/gen/ruby/lib/todo/board.rb)); the test is abridged from [scenarios.slg](pocs/todo-app/src/todo/tests/scenarios.slg).

> **Status: early, `As of 2026-11-24`.** The present tense below describes the design. What runs: a first engine in [`src/`](src/README.md) parses every spec-lang block in these docs, and checks, lints and tests programs, and compiles them to Rust, TypeScript, Ruby and C++, judged on 41 programs including the [todo POC](pocs/todo-app/README.md) and the [platformer](pocs/platformer/README.md). The output is correct there and not optimised. Forms the engine does not run are refused. Everything else is design.

> **If you are a coding agent**, read the [language card](docs/idea/01-language-card.md) first: the whole language on one page.

## Why

Coding agents should work on logic. Most of their time and machine goes to the environment around it.

- Every project needs compilers, interpreters, a file system, a network and databases before one rule can be checked. So agent machines need a lot of memory and fast processors, for work that is mostly not the idea.
- Checking a rule means running lint, types, builds, test suites and CI. Well-tuned projects on good frameworks still wait.
- Agents rebuild the same wheels: another retry, another permission check, another invite flow, another way to page a list.

**Theory, compiled for the real world.** **spec-lang virtualises the world.** A virtual machine stands in for hardware; here the engine stands in for the database, disk, network, clock and everyone else, in memory. Logic is written and checked there, and meets the real world once, on the way to production:

```text
idea (human or AI) -> logic in spec-lang, checked in memory -> CI compiles it and tests the real thing -> production
      think              meant for many times a minute                  once per landing
```

Wheels are shared as ideas that carry their own tests ([packages](docs/idea/35-packages.md)), so an agent picks one up and does not rebuild it.

## What it is

Every program is logic: something is true, something happens, something else becomes true. Around it sits plumbing: schemas, routes, clients, retries, wiring, and the same check repeated in five layers. spec-lang keeps the logic and derives the rest.

```slg
type Seat = Free | Held(by: Id[User], until: Time) | Sold(to: Id[User]);
type SeatNo = 1..=200;
state seats: Map[SeatNo, Seat];

@why("a buyer gets ten minutes to pay")
@uses(seats.write, clock.read)
@guard(open(seats[n], now()) else Taken)
pub rule hold(by: Id[User], n: SeatNo) -> Time = {
  let until = now() + 10min;
  seats[n] = Held(by, until);
  return until;
}

@given(clock.at(0s), seats[1] -> Free)
test ana_holds = {
  hold(ana, 1) -> 10min;
  hold(bob, 1) -> fail Taken;
}

law sold_is_final(t: Trace[hold], n: SeatNo) =
  t.steps.all(it.before[n].is(Sold(_)) implies it.after[n] == it.before[n]);
```

| You write | The compiler derives |
|---|---|
| types and what they mean | schema, migrations, parsing, validation |
| rules, flows, algorithms | transactions, races, retries, jobs, error mapping |
| who may do what | checks on every surface |
| examples, properties, laws | test suites for the interpreter and every backend |
| budgets and deadlines | layout choices, monitors, deploy configuration |

## Three names

| Name | Is |
|---|---|
| **spec-lang** | the language |
| **spec engine** | an engine for ideas: the interpreter that runs logic and tests in memory, with no real sleep, file, network or database. Time, faults and chance are inputs it controls ([06](docs/idea/06-logic-and-tests.md), [decision 006](docs/decisions/006-interpreter-first-in-rust.md)) |
| **backends** | what turns a tested module into real software |

## Where it sits

```text
idea (human or AI) -> spec-lang (AI) -> software                -> machine code, or an
many meanings         one meaning       TS, Ruby, Python, Rust,    interpreter such as
                                        Swift, SQL, frameworks     Ruby, Python or a browser
```

The aim is the highest level that still has exactly one meaning. The first arrow is where guessing happens, so tests, laws, the compiler's questions and human approval all sit there. The later arrows are deterministic. A backend may be compiled or interpreted; either way it runs the same tests and refuses what it cannot keep.

## The point: a shorter dev cycle

| Today | Designed here |
|---|---|
| spread a rule over schema, service, route, client, job, tests | write it once |
| compile, boot, migrate, seed, click | run the logic; nothing is started |
| `sleep`, poll, retry flaky tests | simulated time; same seed, same result |
| agents queue for one database, one compiler, one port | every test has its own world, on any core, in any order |
| "something failed in CI" | the stage says whose fault it is: logic, backend or world |
| an error message | the failing input and a checked edit |
| format, lint, guard scripts | one format; most rules are types |

### Slim locally, heavy in CI

```text
 agent's machine            CI                           production
 one binary            ->   toolchains, databases,  ->   the built software
 logic + tests              devices, browsers
 no services                same tests, real systems
      ^                          |                            |
      |   a replay, run locally  |    a candidate test        |
      +--------------------------+----------------------------+
```

No package install, container, database, browser or compiler where the agent works. The agent gets the logic right in a short loop and moves on; CI checks that the real thing agrees. When it does not, CI sends back a replay: the failing input and what the real system answered, as a red test the agent runs locally.

**The bet:** the bugs worth an agent's time are mostly in the logic: a missed edge, a wrong guard, two things racing, a rule nobody wrote down. If so, most are found before anything is built. Backend, hardware and look-and-feel bugs remain, and how the two divide is the first thing to measure ([19](docs/idea/19-density-and-experiments.md)).

Fewer lines is the means. None of this has been timed: the design goal is feedback in milliseconds for a module, and a model's own thinking time will still be most of the wait. Real databases, devices and screens are tested after the logic is green, and stay as slow as they are ([06](docs/idea/06-logic-and-tests.md), [22](docs/idea/22-lifecycle.md)).

**The short analogy:** TypeScript is a front end whose back end is JavaScript. spec-lang is meant to be a front end with many back ends, frameworks included, and the logic and tests live in the front end. The difference that makes it harder: TypeScript's meaning is JavaScript's, while every spec-lang backend has to match spec-lang's meaning and is tested against it.

## Examples

Everything below parses with the grammar. The outputs show what the interpreter is **designed** to print; it does not exist yet.

### Move money, and be unable to lose it

```slg
type Currency = USD | EUR;
type Cents = Int where self >= 0;
type Account(currency: Currency, balance: Cents = 0);
state accounts: Map[Id[Account], Account];
const a: Id[Account] = Id(1);
const b: Id[Account] = Id(2);

fn total(m: Map[Id[Account], Account], of: Currency) -> Cents =
  m.values.filter(it.currency == of).sum(x => x.balance);

@idempotent(request)
@uses(accounts.write)
@guard(from != to else SameAccount)
@guard(accounts[from].currency == accounts[to].currency else CurrencyMismatch)
@guard(accounts[from].balance >= amount else Insufficient)
pub rule transfer(request: Id[Request], from: Id[Account], to: Id[Account], amount: Cents) = {
  accounts[from].balance = accounts[from].balance - amount;
  accounts[to].balance = accounts[to].balance + amount;
}

law conserved(t: Trace[transfer], c: Currency) =
  t.steps.all(s => total(s.after.accounts, of: c) == total(s.before.accounts, of: c));

@given(accounts[a] -> Account(USD, balance: 100), accounts[b] -> Account(USD, balance: 5))
test sent_twice_moves_once = {
  transfer(Id(1), from: a, to: b, amount: 30);
  transfer(Id(1), from: a, to: b, amount: 30);
  assert accounts[a].balance == 70;
}
```

Delete the `SameAccount` guard and nothing breaks here, because the two assignments still cancel out. Write them as `let x = accounts[from].balance` and `let y = accounts[to].balance` first, and the law catches it:

```text
E-LAW  conserved
  input     transfer(r1, from: a, to: a, amount: 1)   balance 1
  expected  total 1
  got       total 2
  suspects  body, guard, law
  edit      add  @guard(from != to else SameAccount)      strengthens; checked: types + 1 law + 1 test
```

### A web feature

```slg
use web;

@access(owner -> all)
@tenant(owner)
entity Task(
  owner: Id[User],
  @pii title: Text[1..=200],
  done: Bool = false,
);

@live
@page(20)
pub query mine(by: Id[User]) -> List[Task] =
  Task.filter(it.owner == by and not it.done).sort(it.title);

@at("/")
@access(by in User)
page Today(by: Id[User], tasks: Live[List[Task]] = mine(by)) = column(
  form(create, fields: [title]),
  list(tasks, empty: text("Nothing due"), row: check(it.title, on: it.done)),
);
```

Storage, migration, API, validation, tenant scoping, live updates and the matching checks on every surface come from these lines. What the words leave open is asked, never guessed:

```text
E-OPEN  Task.owner: deleting a User who owns tasks is undecided
  edit   cascade  |  keep  |  refuse
```

### A deadline nobody has to poll

```slg
@edge(Pending -> Accepted, via: accept)
@edge(Pending -> Revoked, via: revoke, by: admin)
@edge(Pending -> Expired, after: 7d)
flow InviteLife(at: Invite.status);

law one_membership(t: Trace[accept | revoke], i: Id[Invite]) =
  t.states.all(s => s.members.values.filter(it.invite == i).size <= 1);
```

Accept and revoke racing at the same instant is a trace the interpreter explores, not a bug someone finds in production.

### A game system

```slg
use game;

@on(tick(60Hz))
@within(0.5ms)
@alloc(none)
@post(pos.at.y >= 0m)
system movement(pos: mut Pos, vel: mut Vel, g: Grounded, dt: Time) = {
  if not g.on { vel.v = vel.v + GRAVITY * dt; }
  pos.at = (pos.at + vel.v * dt).clamp_y(0m);
}
```

`vel.v + dt` is a unit error. An allocation in the body is a type error. Replaying the same inputs gives the same state.

### A kernel page allocator

```slg
use kernel;

@uses(used.write, irq.off)
@fails(OutOfMemory)
@post(not old(used).test(result / 4KiB))
pub rule alloc() -> own Frame = machine {
  let word = used.words.position(it != 0xFFFF_FFFF_FFFF_FFFF) else fail OutOfMemory;
  let bit = used.words[word].invert().trailing_zeros();
  used.words[word] = used.words[word] bor (1u64 shl bit);
  return frame_at(word * 64 + bit);
}

law never_shared(t: Trace[alloc | release]) =
  t.states.all(s => s.live(Frame).distinct().size == s.live(Frame).size);
```

Same language, same laws, same tests, down to the bits. `own` makes a double free a type error.

### Think first, fill in later

```slg
@cases((100USD, 3d) -> 100USD, (100USD, 30d) -> 90USD)
fn price(base: Money[USD], early: Time) -> Money[USD] = ?price;
```

A strong model writes the types, the cases and the laws. A small model fills `?price`, and the cases say whether it is right. More: [whole programs](docs/idea/24-examples.md), [one program per domain](docs/idea/15-domain-tour.md).

## Platform independent

A module names no language, framework, database, operating system or chip. Integers, text, time, order and rounding mean the same everywhere, so the logic and its tests run unchanged in the interpreter on any machine, and on every backend that keeps the meaning. Only three places may know a platform:

| Place | Knows |
|---|---|
| `app` | which targets, places and handlers |
| `foreign` | one symbol in one language, behind a contract |
| `machine` | layout, memory order, registers |

The same goes for the author's machine: macOS, Linux or Windows, one binary and the source. Tests use no real files or clock, so paths, line endings and time zones cannot change a result. Building for a platform still needs that platform's tools, such as a Mac for iOS, and those live in the build pool. The interpreter's own suite has to pass on all three systems from its first release for this to hold.

Independent meaning does not promise equal speed or equal reach: a backend that cannot keep a guarantee refuses to build, and a phone screen is still not a console screen.

## The ideas

Design goals. None has been demonstrated.

1. **One core form.** State + event -> new state + effects + reply. A tap, a game tick, a syscall and a database write are all rules.
2. **Tests beside the logic.** Examples, scenarios, properties and laws in one shape. Guard boundaries are derived. Unfinished programs run up to their holes.
3. **Languages are intermediate.** TypeScript, Rust, C++, Swift and SQL are build output, as assembly is to C. Frameworks are backends: [Ultimate](https://github.com/developerz-ai/ultimate) first, [anygame](https://github.com/developerz-ai/anygame-engine) once it has an engine to target.
4. **Four altitudes.** Declare, define, machine, foreign: one type system from a form to an interrupt handler, so there is no wall.
5. **No model in the build.** Models write source; compilation is deterministic.
6. **Evidence is never rounded up.** Proven, bounded, sampled, assumed, stated: every claim says which, and what it trusts.
7. **Errors teach.** The failing input, the checked edit, the one line of the card you need, so that a small model might be productive.
8. **Minimal and sufficient.** If two behaviours are possible, the compiler asks instead of guessing.

## Where it fits, and where it does not

A prediction from the [corpus](docs/idea/16-corpus.md) and the trials; nothing has run.

| Fit | Where | Why |
|---|---|---|
| **Best** | money, ledgers, orders, inventory | conservation, idempotency and races are the hard part |
| **Best** | lifecycles: invites, approvals, subscriptions | one flow, with actors and deadlines |
| **Best** | records with permissions across storage, API, UI and agents | the same rule is repeated in every layer today |
| **Best** | webhooks, retries, jobs | webhooks, the one feature measured, took 2.7x fewer tokens |
| **Good** | many scheduled jobs | the wiring repeats; a single retention job is easier in SQL |
| **Good** | game rules, simulations, protocols | not much shorter; logic runs without building the engine, with determinism and units checked |
| **Good** | one domain, many targets | no hand-synced types; pays only with a second backend |
| **Later** | device control, drivers, storage engines, kernels | safety and crash testing; about the size of Rust |
| **Weak** | one-off queries and scripts, plain algorithms | SQL and Python are already short |
| **Weak** | integrations with odd third-party APIs | the quirks are the work |
| **Not for** | visual design, data exploration, model training | taste is not a law; those ecosystems win, behind `foreign` |
| **Not for** | SIMD kernels, boot code, lock-free internals | hand-written behind a contract |
| **Not for** | a small app that already works | rewriting costs more than it saves |

**Break-even.** The language card costs about 4,900 tokens to load; the measured feature saved about 4,200. One feature of that size does not pay for the card, so the gain must come from several features per session, caching, or fewer wrong attempts. The last is unmeasured.

Rule of thumb: if the same rule would be written in three or more places, or getting it wrong costs money, data or safety, use it.

## What we know so far, and what we do not

| Finding | Caveat |
|---|---|
| One real feature (webhooks in Ultimate's example app): 6,705 tokens of TypeScript, 2,509 of spec-lang, **2.7x** | one feature, no UI, parsed but never run ([12](docs/idea/12-frameworks.md)) |
| Two models given only the card wrote 20 modules: **16 parsed first try** | syntax only; the first author marked 183 lines as guesses. A small model's later 10 of 10 is not comparable: its card held near-answers ([02](docs/idea/02-syntax.md)) |
| In a private corpus, one opinion per person per item took about 2k lines in about 50 files; 10 to 20% of the production code was judged essential | one reader's judgement, one organisation ([16](docs/idea/16-corpus.md)) |
| Maths symbols often cost more tokens than words, and only `² ³ ± °C` cost fewer | tokens are not accuracy; no current Claude tokenizer was measured ([02](docs/idea/02-syntax.md)) |
| Simulations, codecs and kernels are expected to barely shrink | unmeasured; there the gain would be checking, not size ([14](docs/idea/14-systems.md), [16](docs/idea/16-corpus.md)) |

Where to go next: [structure a program](docs/idea/26-program-structure.md) · [who may do what](docs/idea/27-authorization.md) · [standard library](docs/idea/28-standard-library.md) · [first vocabularies](docs/idea/29-first-vocabularies.md) · [type rules](docs/idea/30-type-rules.md) · [a little backend code](docs/idea/31-dropping-down.md) · [build layout](docs/idea/32-build-layout.md) · [algorithms](docs/idea/33-data-structures-and-algorithms.md) · [hygiene](docs/idea/34-hygiene.md) · [packages](docs/idea/35-packages.md)

The first milestone tries to kill the idea: the same semantics embedded in TypeScript is the control. If the bespoke language does not beat it, we ship the embedded form ([19](docs/idea/19-density-and-experiments.md), [20](docs/idea/20-roadmap.md)).

## License

[MIT](LICENSE)
