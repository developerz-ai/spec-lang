# Memory, concurrency and distribution

> **Design.** Nothing here runs yet. This doc covers mutability, compile-time evaluation, hardware memory, waiting, parallelism and distribution: what the author states and which mechanism the compiler then picks.

**Rule: the author states facts; the compiler picks mechanisms.** Nobody writes `constexpr`, `volatile`, `async`, a lock or a thread pool; each follows from a fact already in the source. Syntax is the [language card](01-language-card.md); claims and evidence are [03-types-and-claims](03-types-and-claims.md) and [04-evidence](04-evidence.md).

## Facts and mechanisms

| Fact in the source | Mechanism the compiler may pick |
|---|---|
| a `fn` (pure) | evaluate in the build, cache, reorder, vectorise, run on many cores, run on a GPU |
| a `rule` (atomic, never waits) | a transaction, a single-writer queue, a lock, an interrupt handler with interrupts off |
| a `task` (waits, many commits) | a thread, a coroutine, a durable workflow, a job queue |
| `@mmio(addr)` | volatile access, exact width, no merging |
| `@place(x)` | a network call, serialisation, a generated client |
| `Replicated[T, c]` | a merge function, a log, a consensus round |

## Mutability

| Thing | Can change | Where |
|---|---|---|
| a value, a `let`, a `const`, a parameter | never | everywhere |
| a `var` | inside its block | never escapes; cannot be captured by a lambda |
| a `state` | only inside a `rule`, atomically | the one shared, mutable thing above machine altitude |
| a `mut` borrow | through one unique reference | `machine` bodies |

There is no `const` method and no interior mutability. `a.with(x: 1)` is a copy in meaning; the backend updates in place when the old value is dead. A cache is not mutation the author manages:

```slg
const CRC: List[u32] = (0..<256).map(crc_entry);

@memo
fn route(a: Stop, b: Stop) -> List[Stop] = ?route;
```

`@memo` is legal only on a pure `fn`, so the cache cannot change meaning. Its size and eviction are a policy with a default.

## Compile time without a keyword

A pure expression whose inputs are constants is computed in the build. `CRC` above becomes a table in the binary. There is no marker to propagate through callees.

| Question | Answer |
|---|---|
| limit | a step and memory budget, counted, not timed, so the outcome is the same on every machine |
| over the limit | the value is computed at start-up instead, and the build says so. It is an error only where the value is needed in the build: a `const`, a range bound, a `layout` size |
| numbers | exact arithmetic and the correctly rounded `Float` operations give the build the same bits as the target; a `fn` using `float.native` is not evaluated in the build |
| target facts | layout sizes and word widths are the target's, never the host's |

## Hardware memory and atomics

C and C++ `volatile` mixes two unrelated facts. spec-lang separates them.

| Fact | Write | Meaning |
|---|---|---|
| this place is a device | `@mmio(addr)` on a `layout` | every read and write happens, once, in source order, at the declared width. A read is an effect, because it may change the device |
| this place is shared between contexts | `Atomic[T]` with `load` and `store` | each access names its ordering: `relaxed`, `acquire`, `release`, `seq`. There is no default |

```slg
use kernel;

@mmio(0x4000_0000)
layout Uart(data: u8, status: u8, _pad: Bytes[2], baud: u32);

state ring: Ring[u8, 64];

@on(irq(5))
@alloc(none)
@within(20us)
rule uart_rx() = machine {
  let h = ring.head.load(relaxed);
  if h - ring.tail.load(acquire) < 64 {
    ring.slots[h band 63] = Uart.data.read();
    ring.head.store(h + 1, release);
  }
}

law ring_never_overfull(t: Trace[uart_rx | drain]) = t.states.all(s => s.ring.head - s.ring.tail <= 64);
```

Both exist only at machine altitude. `own`, `ref` and `mut` rule out data races on ordinary memory; atomics are the declared exception, and claims about them are checked by exploring interleavings under a named memory model (`bounded`, trusting that model).

## Waiting: no coloured functions

`async` splits a language in two because waiting is tracked on functions and spreads to every caller. Here waiting is an effect that only one kind may have.

| Kind | Waits | Can call |
|---|---|---|
| `fn` | no | `fn` |
| `rule` | no | `fn`, `rule`; it emits an `event` to start anything slower |
| `task` | yes | `fn`, `rule`, `task`, the outside |

Calls look the same everywhere: no `await`, no future type. Concurrency inside a task is structured:

```slg
@uses(http.call)
@fails(NoQuote)
@within(3s)
task best_quote(by: Id[User], trip: Trip) -> Quote = {
  let both = all(profile: load_profile(by), fares: fare_table(trip));
  let q = race(quote(carrier_a, trip), quote(carrier_b, trip), timeout(2s)) else fail NoQuote;
  return q.with(discount: both.profile.discount);
}
```

| Form | Result | When a branch fails | The others |
|---|---|---|---|
| `all(a, b)` | every reply, by name | the first failure in **source order** is the result | cancelled |
| `race(a, b)` | the first success | ignored unless all fail | cancelled when one wins |

1. **No branch outlives its block.** There is no detached spawn. Work that must outlive the task is an `event`.
2. **Cancellation is delivered at a wait or between steps**, never inside a rule. A rule commits or does not.
3. **Cancellation is not rollback.** A cancelled branch keeps every rule it already committed. If that matters, the branch is a saga (below).

## Parallelism by purity

Nothing is written. A pure function applied across a collection may run in any order on any number of cores, because no order is observable. Three places where order would leak are closed by definition:

| Operation | Defined result | Consequence |
|---|---|---|
| `xs.map(f)` where some `f(x)` fails | the failure of the earliest element | later work is wasted, never visible |
| `sum`, `min`, `max` on `Int`, `Money` | exact, so any grouping agrees | free to split |
| `sum` on `Float` | a fixed pairwise tree whose shape depends only on the length | the same bits on one core or sixty-four; also a smaller error than left to right |
| `fold(init, f)` | strictly left to right | sequential unless `f` is proven associative |

```slg
fn kinetic(bodies: List[Body]) -> Float[J] = bodies.map(b => 0.5 * b.mass * b.v.dot(b.v)).sum();
```

**SIMD** is a backend choice with no source form; it is allowed exactly where the bits do not change. **GPU** is a target: `@target(wgsl)` accepts a pure `fn` with bounded loops, no recursion and `@alloc(none)`, all structural checks. A GPU does not promise exact floats, so the types there carry a bound (`x ± e`) and exact code is rejected.

## Rules interleave: actors without mailboxes

A rule is atomic and rules interleave: the whole concurrency model above machine altitude, explored by test traces.

- **Footprint.** From `@uses`, guards and index expressions the compiler derives what each rule reads and writes, down to the key: `seats[n]`, not `seats`.
- **Mechanism.** Rules with disjoint footprints run in parallel. Overlapping ones are serialised by whatever the backend has: a transaction, a row lock, one queue per key (an actor), or interrupts off.
- **No locks in source.** Deadlock can exist only in `machine` bodies.
- **Events are the messages.** `@on(event)` is delivered at least once after the emitting rule commits; `@idempotent` makes redelivery safe.

## Place

```slg
@place(client)
fn preview(cart: Cart) -> Money[USD] = cart.lines.map(it.price * it.count).sum();

@place(server)
@access(user -> all)
pub rule place_order(by: Id[User], cart: Cart) -> Id[Order] = ?place_order;
```

| Crossing a place boundary | Rule |
|---|---|
| failure | the call gains `Unreachable`, and the caller must handle it; it is never hidden behind a retry |
| waiting | a reply needs a `task`. A `rule` can only send after commit |
| trust | arguments arrive untrusted and are guarded again ([05-effects-and-trust](05-effects-and-trust.md)); the server recomputes `preview` |
| atomicity | no rule spans two places. "All or nothing" across places is a saga |
| time | each place has its own clock. Two readings from different places compare only through a declared skew: `5s ± skew`. Order across places comes from causality, not timestamps |

## Replicated state and invariant confluence

```slg
state likes: Replicated[Map[Id[Post], Set[Id[User]]], causal];
state stock: Replicated[Map[Sku, Int where self >= 0], linear];

rule like(by: Id[User], post: Id[Post]) = {
  if not (post in likes) { likes[post] = Set(); }
  likes[post].insert(by);
}

@guard(stock[sku] >= n else SoldOut)
rule take(sku: Sku, n: 1..=10) = { stock[sku] = stock[sku] - n; }
```

The consistency level is a label (`eventual`, `causal`, `linear`) that joins through expressions like any other, so a value read at `causal` cannot discharge a claim that needs `linear`.

For each rule and each claim it must keep, the compiler asks one question: **if two copies each apply the rule from the same valid state and then merge, is the result still valid?**

| Rule | Answer | Result |
|---|---|---|
| `like` | yes: set union keeps every claim | runs on any copy, no coordination |
| `take` at `causal` | no: two copies each take the last unit | refuted, with the two calls as the counterexample |
| `take` at `linear` | not asked | one order for all copies; the rule waits for a quorum, so callers in other places see `Unreachable` during a partition |

Merge functions for sets, counters, maps and registers come with the library types, their laws (associative, commutative, idempotent) checked once.

## Sagas are tasks

```slg
@uses(http.call, orders.write)
@idempotent(id)
task checkout(by: Id[User], id: Id[Order]) = {
  reserve(id);
  match try_call(charge(id)) {
    Ok(receipt) => { mark_paid(id, receipt); }
    Err(e) => { release(id); }
  }
}

law stock_conserved(t: Trace[reserve | release | mark_paid]) =
  t.states.all(s => s.on_hand + s.reserved + s.sold == TOTAL);
```

Each rule commits alone, so every law must hold **between** steps, not only at the end. That forces an explicit in-between state (`reserved`). After a crash the task resumes after its last finished step; a step cut off mid-call is retried with the same key.

## What others got right

| Source | Kept | Changed |
|---|---|---|
| C++ | `constexpr`: the same language at compile time. Explicit memory orderings | no keyword to propagate; `volatile` split into `@mmio` and atomics |
| Rust | aliasing or mutation, never both; data-race freedom from types | visible only in `machine` bodies; no `async` colour |
| Go | blocking calls with no colour; cheap concurrent work | no shared mutable memory; branches end with their block |
| Erlang | isolated state, messages, failure as a normal value, remote calls that can fail | no untyped mailbox; the footprint decides the queue |
| Koka | effects as rows; waiting as an effect a handler interprets | only one kind may hold the waiting effect, so rows stay short |
| Trio, Kotlin | a child never outlives its scope; cancellation flows down | the only forms are `all` and `race` |
| CRDT and coordination-avoidance research | merge laws; coordinate only where an invariant demands it | derived from the program's own laws, with a counterexample |

## Limits

- **Three kinds are a colouring.** A `rule` cannot call a `task`. It is coarser than `async`, and it marks a real difference (atomic or not), but turning a rule into a task is still a change every caller's atomicity depends on.
- **Footprints can be too coarse.** A rule that scans a whole map conflicts with every writer.
- **The fixed-tree `Float` sum costs speed** against an unordered reduction, and vectorised transcendental functions are not bit-identical across targets.
- **Invariant confluence is a solver question** and will come back undecided for rich state; the safe default is to coordinate, which costs availability.
- **Memory-model checking is bounded** and trusts the named model.
- **Clock skew bounds are assumed.** Whether a value is computed in the build depends on a budget number and can change as code grows.
