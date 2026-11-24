# Logic and tests

> **Design.** Nothing here runs yet. The inner loop: idea, logic, test, counterexample, fix, with no target chosen and no test glue.

**Rule.** The shortest path from an idea to a verdict is the product. Everything else in this repository serves it.

```text
idea -> types + rules (holes allowed) -> tests -> interpreter -> counterexample or open question -> edit -> again
                                                                  green -> prove -> build -> software
```

## One turn of the loop

The idea: move money between accounts, never create any.

```slg
type Cents = Int where self >= 0;
state accounts: Map[Id[Account], Cents];

rule deposit(to: Id[Account], amount: Cents) = {
  accounts[to] = (accounts[to] else 0) + amount;
}

@guard(amount <= accounts[from] else Insufficient)
rule transfer(from: Id[Account], to: Id[Account], amount: Cents) = {
  let a = accounts[from];
  let b = accounts[to];
  accounts[from] = a - amount;
  accounts[to] = b + amount;
}

law conserved(t: Trace[deposit | transfer]) =
  t.steps.all(it.event.is(transfer(_, _, _)) implies it.after.values.sum() == it.before.values.sum());
```

The edit that added `transfer` returns, in the same response:

```json
{"code":"E-LAW","law":"conserved","decl":"transfer@b:41f0",
 "counterexample":{"replay":"c:7d","shrunk":[9,2],
  "trace":["deposit(#1, amount: 1) -> ok","transfer(#1, to: #1, amount: 1) -> ok"],
  "before":{"#1":1},"after":{"#1":2}},
 "question":{"code":"E-OPEN","ask":"a transfer to the same account",
  "options":[{"does":"refuses","edit":"@guard(from != to else SameAccount)"},
             {"does":"changes nothing","edit":"if from == to { return; }"}]}}
```

The law found a bug nobody wrote a case for. Two fixes satisfy it, so the compiler **asks**. The author answers with one guard:

```slg
@guard(from != to else SameAccount)
@guard(amount <= accounts[from] else Insufficient)
rule transfer(from: Id[Account], to: Id[Account], amount: Cents) = {
  accounts[from] = accounts[from] - amount;
  accounts[to] = accounts[to] + amount;
}

@given(accounts[Id(1)] -> 5)
test no_self_transfer = {
  transfer(Id(1), to: Id(1), amount: 1) -> fail SameAccount;
}
```

Over `Trace[transfer]` alone no account could exist and the law would hold of nothing: `E-VACUOUS` ([08](08-approval-and-anti-cheating.md)), not a pass.

## Three things to test

| Subject | Question | Derived by the interpreter | Written by the author |
|---|---|---|---|
| **logic** | is the new state right? | values from types, shrinking | scenarios, properties, laws |
| **guards** | is each refusal reached, and where is its edge? | last accepted and first refused value of every comparison | which error, and why the line is there |
| **assumptions** | what breaks if the world is not as assumed? | a fault per effect the code uses | what must still hold |

Plumbing is not on the list. It is generated, and its backend has a conformance suite. Because guards, flows and claims are data, the interpreter knows what nobody exercised:

```json
{"uncovered":{
  "guards":[{"rule":"accept","guard":"now() < until","edge":{"lastAccepted":"7d - 1ms","firstRefused":"7d"},"missing":"firstRefused"}],
  "branches":[],
  "edges":[{"flow":"InviteLife","edge":"Open -> Revoked"}],
  "assumptions":[{"task":"capture","fact":"the provider dedupes by key","fault":"reply lost","run":false}],
  "refusals":[{"rule":"transfer","error":"Missing"}]},
 "derivedOnly":["transfer.Insufficient"]}
```

A derived case shows **where** the line is, never that it is right, so `derivedOnly` adds no independent evidence ([08](08-approval-and-anti-cheating.md)). `1ms` is the clock handler's grain.

## Five forms, no framework

| Form | Asks | Written as |
|---|---|---|
| table | do these inputs give these values? | `@cases(...)` on a `fn` |
| scenario | does this story go this way? | `@given(world)` then calls with `->` |
| context | and from there? | `@given(other_test)` |
| property | for all inputs of these types? | parameters on the test |
| trace | in every order, with faults? | a `Trace[...]` parameter |

A law is a property or a trace that is part of the interface. One claim is a test oracle, an input generator, a proof obligation and, where asked for, a monitor ([04](04-evidence.md)).

## Cucumber without glue, RSpec without setup

Cucumber's good idea is a story a non-programmer can approve. Its cost is a second program: a regex and a step function for each sentence. Here the steps are calls:

```slg
type Invite = Pending(until: Time) | Accepted(by: Id[User]);
state invites: Map[Token, Invite];
state members: Set[Id[User]];
const ana: Id[User] = Id(1);
const bob: Id[User] = Id(2);
const t1: Token = Token("t1");

@guard(not (t in invites) else Exists)
rule invite(t: Token) = { invites[t] = Pending(now() + 7d); }

@why("an invite works once, for seven days")
@guard(invites[t].is(Pending(until)) and now() < until else Gone)
rule accept(by: Id[User], t: Token) = {
  invites[t] = Accepted(by);
  members.insert(by);
}

@given(clock.at(0s), invites[t1] -> Pending(7d))
test ana_accepts = {
  clock.advance(6d);
  accept(ana, t: t1);
  members -> Set(ana);
}

@given(ana_accepts)
test second_use_is_refused = {
  accept(bob, t: t1) -> fail Gone;
  members -> Set(ana);
}

@given(clock.at(0s), invites[t1] -> Pending(7d))
test closed_from_day_seven(late: 7d..=30d) = {
  clock.advance(late);
  accept(ana, t: t1) -> fail Gone;
}

law joined_by_invite(t: Trace[invite | accept | crash]) =
  t.states.all(s => s.members.all(m => s.invites.values.contains(Accepted(m))));
```

| Taken | Rule here |
|---|---|
| Cucumber: the readable story | the English is **rendered** from the calls, never parsed |
| RSpec: nested context | `@given(test)` starts from that test's end state, replayed, never shared. A cycle is an error |
| RSpec: one fact per expectation | each `->` is reported by itself |
| property testing: shrinking | on the trace: drop steps, then faults, then values |

If a parent fails, its children report `blocked by ana_accepts`: one cause, one failure.

A `@given` world that breaks a law or flow is an error (`E-WORLD`).

The approval rendering of the two scenarios:

```text
accept: "an invite works once, for seven days"
  Given the time is 0 and invite t1 is Pending until day 7
  When 6 days pass and ana accepts t1
  Then members are exactly {ana}
    From there: when bob accepts t1, it is refused: Gone. Members are exactly {ana}
```

## The world is handlers, so nothing is mocked

| Handler | Controls | Faults it can inject |
|---|---|---|
| `clock` | `at`, `advance`; fires due `tick` and `every` rules | skew, pause |
| store (derived from `state` and `entity`) | contents | conflict, crash at any commit boundary |
| `net`, `http` | `reply`, `calls` | drop, delay, duplicate, reorder, partition |
| `disk` | durable and volatile contents | torn write, lost flush |
| scheduler | which enabled rule or task step runs next | every order the triggers allow |
| `crash` | where execution stops and restarts | between any two steps, inside any outside call |
| `random`, `Id` | seed | none |

The default world is simulated and seeded, so every run replays. A handler is **an assumption about the real thing**. Each ships traces to replay against it; until they pass, results resting on it say `sampled, sim: unconformed`.

## Testing a task

A `rule` is atomic: nothing to interrupt. A `task` has known steps, so for **every** task the interpreter enumerates three faults with nothing written: a crash at each gap and inside each call, a lost reply, a duplicate trigger. The author writes what must survive.

```slg
const provider: Url = url"https://pay.example.com";
event ordered(order: Id[Order]);
state receipts: Map[Id[Order], Receipt];
state failed: Set[Id[Order]];

rule record(order: Id[Order], r: Receipt) = { receipts[order] = r; }

rule give_up(order: Id[Order]) = { failed.insert(order); }

@on(ordered)
@retry(5, backoff: 2s, on: Unreachable, exhausted: give_up)
@idempotent(order)
@uses(http.call, receipts.write)
task capture(order: Id[Order]) = {
  let reply = http.call(provider, body: order) else fail Unreachable;
  record(order, r: reply.body);
}

law settles(t: Trace[capture | crash], o: Id[Order]) =
  t.steps.any(it.event == capture(o)) implies t.eventually(o in it.receipts or o in it.failed);

@given(http.reply(200), net.drop(1))
test reply_lost_after_charging = {
  capture(Id(1)) -> fail Unreachable;
  capture(Id(1));
  assert http.calls.size == 2 and http.calls[0].key == http.calls[1].key;
  assert receipts.size == 1;
}

@given(http.reply(200))
test duplicate_trigger = {
  capture(Id(1));
  capture(Id(1));
  assert http.calls.size == 1;
}
```

| Fault | Derived run | Reported when the law holds only because |
|---|---|---|
| crash after the call, before `record` | restart; the call repeats with the same key | the provider is assumed to dedupe by key |
| lost reply | the same, through `@retry` | the same |
| duplicate trigger | second run replays the first reply | `@idempotent`; without it, the second charge is the counterexample |
| crash between two rules | the state between them is visible to other rules | no law forbids that state |
| `all` or `race` in a task | every completion order | no reply depends on which finished first |

Crash points are enumerated up to a small trace, so this is `bounded(k)`. "The provider dedupes" stays `assumed`. Without `exhausted: give_up` the law is refuted by six unreachable replies: an order that is neither received nor failed.

## Holes run

`?name` type-checks, runs until reached, then yields `Blocked` with what was known. A blocked test is never green, and never hides the others.

```json
{"hole":"release.?release","type":"()","uses":[],
 "reached":[{"by":"lapsed_hold_is_released","live":{"n":1,"seats[1]":"Held(#1, 10min)","now":"10min"},
   "needs":"seats[1] == Free","how":"solved"}],
 "notReached":["sold_is_final: 2 of 3 branches"],"base":"b:9c1"}
```

Where the later expectations are finite or solvable, the runtime works backwards to the value or write that would make each test pass. So the failing tests *are* the hole's specification. Where it cannot, it says `needs: unknown`.

This splits the work by strength. A strong model or a person writes types, guards, laws and tests and leaves holes. A small, cheap model fills them: each is one expression, with a type, allowed effects, live values, the required result, and an error that carries the edit ([07](07-agent-loop.md)). Filling is an ordinary edit, the filler's role cannot touch laws or tests ([08](08-approval-and-anti-cheating.md)), and the compiler never calls a model. Whether it pays is an arm of the experiments ([19](19-density-and-experiments.md)).

## Run almost nothing

Goal, unmeasured: verdicts in milliseconds for a module. The mechanism is not a fast evaluator:

```text
key = hash(test, interpreter, seed, handlers, every declaration it evaluated,
           every lookup that found nothing)
```

A result is reused when its key is unchanged. The last line matters: a new declaration can change what a failed lookup now finds. Determinism makes reuse sound. A property stores which seeds it has tried; idle time tries new ones. Every shrunk counterexample is kept and replayed first.

## Tests that are not written

In the [corpus](16-corpus.md), test code outweighed production code in every case that has tests, from about even to nearly four times. Most assertions read were meaningful; what repeated was the harness and **the same rule at every layer**.

| Today | Here |
|---|---|
| the rule in the domain, service, route and client tests | one scenario; each surface of the rule runs it through its own door |
| fakes, fixtures, builders, wiring | handlers and `@given` |
| serialise, parse, paginate, retry tests | the backend's own suite |
| race and crash tests by hand | trace parameter; task faults derived |

## The same tests on every backend

A test yields an observation: replies plus effects. It is lowered to each target and compared: `same`, `diverges` (a backend bug, by definition) or `not transferable` (the target cannot script a needed fault, so evidence there drops visibly). Detail in [11](11-backends-and-parity.md).

## Limits

- **Same author, same blind spot.** Law, test and logic can share one missing idea. [08](08-approval-and-anti-cheating.md) raises ambiguity; nothing invents the requirement nobody asked for.
- Exploring all orders explodes. Traces are bounded, and reported as `bounded(k)` or `sampled`.
- Working backwards through a hole is a solver problem: expect `unknown` beyond small state.
- A simulated service can be wrong in the same way as the author's idea of it. Assumption coverage counts faults the handlers know; an unimagined fault is not uncovered, it is absent.
- Handler controls beyond `clock.advance`, `net.drop` and `http.reply` are proposals.
- Generators for tight claims can find no values; that is an error, not a skip.
- How much test mass this removes is a judgement until the corpus cases are ported.
