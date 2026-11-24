# Language card

> **Design.** Nothing here runs yet except the parser check: every `slg` block in this repo parses with [the grammar](25-grammar.md). Types, names and meaning are unchecked.

The whole language on one page: what a model is given before it writes spec-lang. Everything else in `docs/idea/` explains why. Where another doc disagrees with this one, this one wins.

**The AI writes logic and tests. The interpreter runs them at once. The compiler generates the software.** Programming languages and frameworks are the compiler's intermediate representation. No model runs inside the build.

## One declaration shape

```text
{@note(args)} [pub] kind Name[generics](data) -> Result = body
```

| Part | Holds | Rule |
|---|---|---|
| `kind` | what is declared | core kind, or one a `use`d vocabulary defines; never a keyword |
| `( )` | names you invent: fields, event parameters, quantified variables | data only, never policy |
| `@note(args)` | every claim, policy, trigger and budget | closed set; one typed meaning; an unknown, misplaced or contradictory note is an error |
| `= body` | how | its form is the altitude |

Grammar never grows per domain. A vocabulary adds kinds and notes, nothing else.

## Core kinds

| Kind | Is |
|---|---|
| `type` | a set of values: range, record, sum (`A \| B(x: T)`), or `T where claim` |
| `const` `state` | a fixed value · a place that rules change |
| `fn` | pure computation |
| `rule` | **state + event -> new state + effects + reply.** Parameters are the event. Atomic; never waits on the outside world |
| `task` | steps over many commits: each rule it calls commits alone; may call out and use the reply; resumes after a crash |
| `event` | a fact a rule emits and `@on` listens to |
| `flow` | a state machine every writer of that place must obey |
| `test` | an example, table, property, trace or scenario |
| `law` | a claim that must always hold; part of the interface |
| `note` `kind` `handler` `app` | extend the vocabulary · implement an effect · compose and pick targets |

Every program is rules: a request, a game tick, an interrupt, a syscall, a tap, a reconcile. Domain kinds (`entity`, `query`, `page`, `system`, `service`, `controller`, `layout`) are shorthand for ordinary declarations; `spec-lang view --as expanded` shows them.

## Altitude is the form of the body

```slg
@post(result > x) fn a(x: Int) -> Int;                            // declare: derived, or the build fails
@post(result > x) fn b(x: Int) -> Int = x + 1;                    // define: an expression or a block
@post(result > x) fn c(x: Int) -> Int = machine { return x + 1; } // machine: memory, layout, interrupts
@post(result > x) fn d(x: Int) -> Int = foreign c "fast_inc";     // foreign: evidence starts at assumed
@post(result > x) fn e(x: Int) -> Int = ?e;                       // hole: runs until reached, then Blocked
```

Callers never change when the body does.

## Core notes

| Note | Means |
|---|---|
| `@why("...")` | the reason; tracked to laws and tests, never itself "proven" |
| `@pre(p)` `@post(p)` | caller's duty · promise; `result`, `old(x)` available in `@post` |
| `@guard(p else Err)` | runtime refusal with a typed error, checked before the body; names bound by a pattern in it are usable in the body |
| `@uses(store.write, clock.read)` | effects; inferred when private, written when `pub` |
| `@fails(Err, ...)` | errors the body can raise beyond its guards |
| `@on(trigger)` | what calls it: an `event`, `tick(60Hz)`, `every(6h)`, `http.post("/x")`, `irq(5)` |
| `@within(limit)` `@alloc(none)` | time and memory budgets |
| `@cases(args -> value, ...)` | table test on a `fn` or parameterised `test` |
| `@given(state -> value, handler, other_test)` | starting world of a test; a test name starts from its end state |
| `@retry(n, on: E, exhausted: rule)` `@idempotent(key)` | delivery policy; `exhausted` runs when the last try fails |
| `@pii` `@secret` `@tenant(x)` | information-flow labels, also on parameters |
| `@access(who -> ops)` | who may: `anyone`, `user`, a role, or a field holding an `Id[User]`; with a caller and neither this nor a guard on `by`, **denied** |
| `@edge(A -> B, via: rule, by: who, after: t)` | one legal move of a `flow`; any other fails `IllegalMove`; a self-move restarts `after`; `by` adds to `@access` |
| `@assume("reason")` | admits a claim unproven; always listed in reports |
| `@target(x)` | choose a backend |
| `@memo` `@mmio` `@place` `@allows` | see below; `@allows` says where a new note may go |

## Data

```slg
type Vote = Up | Down;                                     // sum
type Key(user: Id[User], message: Id[Message]);            // record
type Cents = Int where self >= 0;                          // claim; `self` is the value
type Account(currency: Currency, balance: Cents = 0);      // default
state accounts: Map[Id[Account], Account];                 // starts empty
state floor: Int = 0;                                      // initial value

fn demo(a: Account) -> Account = {
  let k = Key(ana, message: m1);                           // construct
  let b = a.with(balance: a.balance + 5);                  // copy with a change
  return b;
}

rule pay(id: Id[Account], n: Cents) = {
  let x = accounts[id] else fail NoAccount;                // missing key: fails unless handled
  accounts[id].balance = x.balance + n;                    // assign inside an entry
  accounts.remove(id);
}
```

| Question | Answer |
|---|---|
| Missing map key | `m[k]` fails with `Missing`; `m[k] else v`, `m.get(k) -> Option`, `k in m`. Never a silent default |
| Empty, literal | `1..=n` iterates; `[]`, `[1, 2]`, `Map()`, `Map(a -> 1)`, `Set(1, 2)` |
| Equality | `==` is structural on every type |
| `Time` or `Duration` | one type, `Time`; `now()` is time since the world began, `10min` is the same unit. Calendar dates are `Date`, instants `Instant`, both in vocabularies |
| Arithmetic | `+ - * /`; `Time / Time` is a number, `Int * Time` is a `Time`; on `Int` use `a.div(b, floor)`, because `/` on `Int` is an open choice |
| Ids, random | `Id.fresh()`, `random.token()`, `random.int(1..=6)`: effects, seeded and replayable in tests |
| Errors | a name after `fail` or `else` declares itself; give it data with `type Held(by: Id[User]);` |
| Integers | `Int` is exact and unbounded; `u8 ... u64`, `i8 ... i64` wrap and belong to layouts. `.round(half_even)` is how `Decimal` and `Money` come back to their scale |

**Core library.** Collections: `size filter map flat_map fold sum min max all any find position distinct sort(key) reversed take drop zip group_by first last contains push insert remove keys values entries`. Text: `size split join trim lower starts_with replace bytes`. Numbers: `abs min max clamp div round`. Option: `Some none else is`. Anything not listed comes from a `use`d vocabulary.

## How it runs

1. **A rule is atomic**: all its writes happen or none; a failure leaves the world unchanged. A rule called inside a rule joins it.
2. Rules interleave; traces explore every order the triggers allow. A trace starts from `@given`, or from any world its types, flows and laws allow; a counterexample that starts where no rule can reach says so and offers the missing law.
3. Guards run first, top to bottom. The first that fails gives the error. Names a guard pattern binds are in scope after it.
   `x else fail E` propagates; `match try_call(f) { Ok(v) => ...; Err(e) => ...; }` handles a failure here.
4. `by` is the caller; a test passes it as an ordinary argument. `@access` is required where someone can call in (http, client, agent tool); timers, events and interrupts have no caller and need none.
5. Replies may be omitted; `return;` leaves early.
6. `@idempotent(key)`: same key and payload replays the first reply; same key, different payload is `fail KeyReused`.
7. In a test, `clock.advance(d)` fires every `every`, `tick` and flow deadline that falls due. `@on` rules can also be called directly.
8. In a `rule`, outside effects (mail, http) are sent after commit, at least once, and have no reply. A reply needs a `task`, started by `@on` or by `name.start(...)` after the caller commits.
   A run has one key. After a crash, finished steps of the attempt are not run again. `@retry` starts a new attempt, which repeats every step with the same key, so a step that is not safe to repeat is `E-OPEN`. A direct call in a test is one attempt. `@idempotent` replays successes only. Exactly once across a third party is `assumed`.
   Events reach each listener after commit, at least once, in its own commit, in the order one rule emitted them.
9. `@within` is a claim, checked where it can be and monitored where it cannot; `over: stop` or `over: refuse` acts on overrun, the default only reports.
10. On an exposed rule, `Missing` and a denied `@access` give the same `NotFound`, so existence does not leak. A direct call in a test sees `Missing`.
11. The local disk, clock and device registers are part of the world; another machine or person is outside.

**Traces.** `t.steps` gives `event`, `reply`, `at`, `before`, `after`; with several states write `it.before.accounts[k]`. `t.states` gives whole states. Liveness: `t.eventually(p)`, `t.within(5s, p)`; they assume fair scheduling and say so.

## Each remaining form, once

```slg
use web;

@edge(Open -> Accepted, via: accept)
@edge(Open -> Revoked, via: revoke, by: admin)
@edge(Open -> Expired, after: 7d)
flow InviteLife(at: Invite.status);

event published(post: Id[Post]);

rule publish(by: Id[User], id: Id[Post]) = {
  posts[id].live = true;
  published.emit(id);
}

@allows(rule, query)
note page(size: 1..=500) = page_policy;

rule purge(n: Int) = {
  for x in notes.values.filter(it.at + 90d <= now()).take(n) { notes.remove(x.id); }
}

@on(every(6h))
@within(30s, over: stop)
task sweep() = {
  while notes.values.any(it.at + 90d <= now()) { purge(500); }
}

@on(published)
rule fan_out(post: Id[Post]) = {
  for e in endpoints.values { deliver.start(post, endpoint: e); }
}

@retry(5, backoff: 2s, on: Unreachable)
@idempotent((post, endpoint.id))
@uses(http.call)
task deliver(post: Id[Post], endpoint: Endpoint) = {
  let reply = http.call(endpoint.url, body: post) else fail Unreachable;
  if reply.status >= 500 { fail Unreachable; }
  record_sent(post, endpoint: endpoint.id);
}

@access(owner -> all, admin -> read)
entity Note(owner: Id[User], @pii text: Text[1..=500]);

@cases((1, 2) -> 3, (0, 0) -> 0)
fn add(a: Int, b: Int) -> Int = a + b;

@given(http.reply(503), clock.at(0s))
test retries_then_stops(n: 1..=3) = {
  for i in 1..=n { deliver(Id(1), endpoint: hook) -> fail Unreachable; }
  assert http.calls.size == n;
}

@on(irq(5))
@alloc(none)
@within(20us)
rule uart_rx() = machine {
  let h = ring.head.load(relaxed);
  if h - ring.tail.load(acquire) < 64 {
    ring.slots[h band 63] = uart.data.read();
    ring.head.store(h + 1, release);
  }
}

handler FrozenClock(at: Time) = { clock.now -> at; }

@target(ultimate)
app Site(rules: [deliver, sweep]);
```

## Expressions

| Want | Write |
|---|---|
| Type arguments, index | `List[Int]`, `seats[n]` (`<` is only comparison) |
| Ranges | `1..=200`, `0..<n` (bare `..` is illegal); a range is also a type |
| Lambda | `it` for one unnested argument, else `x => ...` |
| Choice | `if c then a else if d then b else e` as a value; `if c { } else { }` as a statement; `match x { A => 1; B(n) => n; }`; an arm with statements is a block: `A => { assert ok; }` |
| Test a variant | `x.is(Held(by, _))`: in a pattern, a name already in scope compares, a new one binds (nothing shadows, so never both) |
| Fail | `fail Err`; `x else fail Err`; propagate with `try` |
| Logic | `and or not implies`; `a <= b <= c` means both; a guard's clauses `@guard(p else X, q else Y)` run in order; `s.all(p)`, `s.any(p)`, `s.sum()`, `s.sum(f)` |
| Modulo, bits | `mod`, `band bor bxor shl shr` (`%` is only percent) |
| Units | `10ms 60Hz 4KiB 80°C 70% 19.99USD 5u64`; compound `9.81[m/s²]`; `5m ± 0.1m` |
| Text | `"hi {name}"`, `date"2026-11-24"`, `re"^a+$"`, `url"..."`, `b"..."` |
| Step by step | `let`, `var`, `=`, `for x in xs { }`, `while c { }`, `return`, `assert` |
| Ownership (machine only) | `own`, `ref`, `mut` |

A block's value is its `return`; there is no tail expression. Statements end in `;` or a closing `}`. Name arguments after the first unless the type makes order obvious. No overloading, shadowing, wildcard import, user operator, implicit conversion, null, exception, `+=` or `? :`.

**Maths symbols** `≤ ≥ ≠ ∈ ∧ ∨ ¬ ⇒ → × ÷ µs` are accepted; the formatter prints ASCII, which measured no worse. `² ³ ± °C` stay.

## Memory, time and place

| Fact | Write |
|---|---|
| Immutable | the default; `var` a local, `state` a place; `mut` marks a parameter a rule may change, and is a borrow in `machine` |
| Compile time | nothing: pure with constant inputs is computed in the build, within a counted step budget; host-dependent facts are excluded |
| Cache | `@memo(size: 1000) fn` (pure only; an error with `@alloc(none)`) |
| Hardware place | `@mmio(0x4000_0000) layout Uart(...)`: every access kept, in order |
| Waiting | no async or await; only a `task` waits, and a rule cannot call one: `all(a(), b())`, `race(a(), timeout(5s))`. A losing branch is cancelled, not undone: rules it committed stay |
| Parallel | nothing: pure maps may run in any order, rules stay atomic, no locks. `Float` `sum` uses one fixed tree, `fold` runs left to right, the first failure in source order is the one reported |
| Where it lives | `@place(client)`; a call across places can be `Unreachable`, and its reply needs a `task`. Atomic means within one place |
| Copies | `state x: Replicated[T, causal]`; a stale read cannot feed a claim that needs a fresh one |

## Tests

| Want | Write |
|---|---|
| table | `@cases(...)` |
| property | parameters: values are generated from their types and shrunk |
| scenario | `@given(world)` then steps: `call;` is *when*; `call -> value;` is *when + then*; `call -> fail Err;` is a refusal |
| nested context | `@given(other_test)` |
| trace | a parameter `t: Trace[put \| crash]`; use `t.steps`, `t.states` |
| time, network, crash | handlers: `clock.advance(10min);`, `net.drop(1);` No mocks |

No step definitions: scenarios call rules directly. Given/when/then English is a rendering for human approval.

## Evidence

`refuted` is always an error. Otherwise every claim reports how it is known: **proven > bounded(k) > sampled > assumed > stated**, plus whether it is monitored at runtime and what it trusts. The compiler never rounds up. A `sketch` profile blocks only on type errors and refutations; `release` enforces the project's policy. Evidence is about the meaning; a backend's own fidelity is reported next to it.

## Minimal and sufficient

Leave a word out only if one visible rule recovers it: a type, inference, a named default policy. If two behaviours remain, the compiler asks (`E-OPEN`) and never guesses. Plumbing is never written: storage, routes, clients, parsing, scoping, pagination, retries, error mapping, wiring.

## A whole module

```slg
type Seat = Free | Held(by: Id[User], until: Time) | Sold(to: Id[User]);
type SeatNo = 1..=200;
state seats: Map[SeatNo, Seat];
const ana: Id[User] = Id(1);
const bob: Id[User] = Id(2);

fn open(s: Seat, t: Time) -> Bool = match s {
  Free => true;
  Held(_, until) => t >= until;
  Sold(_) => false;
}

@why("a buyer gets ten minutes to pay")
@uses(seats.write, clock.read)
@guard(open(seats[n], now()) else Taken)
pub rule hold(by: Id[User], n: SeatNo) -> Time = {
  let until = now() + 10min;
  seats[n] = Held(by, until);
  return until;
}

law sold_is_final(t: Trace[hold], n: SeatNo) =
  t.steps.all(it.before[n].is(Sold(_)) implies it.after[n] == it.before[n]);

@given(clock.at(0s), seats[1] -> Free)
test ana_holds = {
  hold(ana, 1) -> 10min;
  hold(bob, 1) -> fail Taken;
}
```

Full programs: [24](24-examples.md) (seats, todo, webhooks, movement, k8s service, PID, page allocator, B-tree + WAL).

## Names

Language and CLI `spec-lang`, files `.slg`, code fence `slg`. Path is the module: `seats/core.slg` is `seats.core`. `use seats.core;` gives `core.hold`; `use seats.core.hold;` gives `hold`; `use web;` gives a vocabulary's kinds and notes. Six verbs: `view edit test prove build land`.
