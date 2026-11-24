# Backends and parity

> **Design.** Nothing here runs yet. What a backend must declare, how the same tests judge every backend, when a backend must refuse, and how target languages are treated.

**Rule: target languages are the compiler's intermediate representation.** TypeScript, Rust, C, C++, Swift, Kotlin, Go, SQL, WGSL and deployment manifests stand to spec-lang as assembly stands to C. Frameworks are higher-level targets of the same kind ([12-frameworks](12-frameworks.md)). Nobody authors them by default, nobody edits the output, nobody approves it as truth.

**Rule: a backend that cannot keep a claim refuses.** It never approximates.

## Who touches target code

- **Application authors** never edit generated files: the build overwrites them and the gate rejects a tree whose hash differs from the plan's. They may write a `foreign` body when trusted code already exists, the thing is below every altitude, or a measured hot path beats every strategy.
- **Backend authors** write printers and runtime shims in the target language.
- **Anyone debugging** may read generated code as one reads disassembly. It signals a suspected backend bug, and the answer is a parity case, never a patch.
- **Leaving**: `spec-lang edit --eject Note` rewrites a body to `foreign`, hands over the last generated files, keeps the derived contract and tests, and resets evidence to what they justify. One-way, and a guarantee diff.

## The backend contract

A backend is a package. The `compiler` vocabulary declares the one kind and the notes it is written with; none of them is core.

```slg
kind backend = register_backend;

@allows(backend) note carries(what: Kind, keeps: List[Claim], survives: CrashScope = process) = carry;
@allows(backend) note refuses(claims: List[Claim]) = refuse;
@allows(backend) note supplies(effects: List[Effect]) = supply;
@allows(backend) note scripts(handlers: List[Effect]) = script;
@allows(backend) note isolation(level: Isolation) = isolate;
@allows(backend) note hosts(lang: Lang) = host;
@allows(backend) note toolchain(tool: Tool) = pin_tool;
```

```slg
use compiler;

@toolchain(bun)
@toolchain(postgres)
@carries(data.entity, keeps: [identity, access, tenant, durable])
@carries(data.query, keeps: [access, page, order])
@carries(sync.live, keeps: [order_per_entity])
@carries(task, keeps: [run_key, attempt, step_once, resume], survives: process)
@refuses(within(under: 1ms), alloc, exactly_once)
@supplies(store, clock, http, job)
@scripts(clock, store, http, crash, store.barrier)
@isolation(serializable)
@hosts(typescript)
backend ultimate;
```

| Note | Declares, and what the planner does with it |
|---|---|
| `@carries(kind, keeps: [...])` | a kind taken whole and the claims kept; a claim not listed is not kept, so the node is infeasible there if policy needs it |
| `@refuses(...)` | claims it can never keep; an argument narrows it, so a `@within` of seconds on a `task` is carried as a timer while a sub-millisecond budget is refused |
| `@supplies(...)` | effects it can implement; a rule using another cannot be placed on it |
| `@scripts(...)` | handlers it can drive deterministically in tests, including installing any start world; decides which tests transfer |
| `@isolation(level)` | the strongest isolation its store really gives; decides how atomic rules are carried (below) |
| `@hosts(lang)` `@toolchain(...)` | languages accepted in `foreign` bodies; pinned tools, part of the backend hash |

A backend is chosen only by an `app` (`@target`), which is also the only place that names places and handlers. A backend never picks itself.

## Atomic rules on a real SQL store

**The meaning:** a rule is atomic, so every outcome must be explained by some one-at-a-time order of the rules that ran. A SQL store at its default isolation does not give that when a guard reads a count or a scan.

```slg
@access(by in members else NotFound)
@guard(bookings.values.filter(it.slot == s).size < slots[s].cap else Full)
pub rule book(by: Id[User], s: Id[Slot]) = {
  bookings[Id.fresh()] = Booking(by, slot: s);
}

law never_overbooked(t: Trace[book], s: Id[Slot]) =
  t.states.all(w => w.bookings.values.filter(it.slot == s).size <= w.slots[s].cap);
```

Two `book` calls can both count `cap - 1`, both insert, and break the law. The compiler sees the risk in each rule's **read set**, computed on SIR-F: point reads by key, and predicate reads (scan, count, `any`, `all`, `min`, `max`) with the columns that partition them. A rule with a predicate read that another write can change needs one of these strategies, tried in order:

| Order | Strategy | Applies when | Emits |
|---|---|---|---|
| 1 | constraint | the guard matches a catalogue shape: "no other row with this key" (unique), "no overlapping range" (exclusion), "parent exists" (foreign key) | the constraint; the in-app guard stays for error order; a violation at commit maps back to that guard's error |
| 2 | conditional write | the predicate is a count or sum per partition key and that key has a row (`slots[s]`) | a counter column kept by every writer, and `update ... set n = n + 1 where n < cap`; zero rows means `Full` |
| 3 | anchor lock | the predicate is partitioned by a key that has a row | `select ... for update` on the anchor row in every rule that reads or changes that partition |
| 4 | serializable retry | the store gives true serializable isolation | the rule's transaction at that level; a serialization failure re-runs the whole rule, bounded |
| 5 | table lock | only when pinned by the author | an exclusive lock for the transaction |

Strategies 2 and 3 are sound only if **every** writer takes part. That is checkable because a module owns its state: the writers are exactly its rules, and the compiler emits them all. Re-running in strategy 4 is safe because a rule's outside effects leave only after commit.

The backend **must refuse** (`E-PLAN-REFUSED`, naming the read set) when:

| Case | Why |
|---|---|
| No catalogue shape fits, no anchor row exists, and `@isolation` is below serializable | nothing left keeps the rule atomic; "repeatable read" or snapshot isolation still allows this anomaly |
| Hand-written code outside the compiler writes the same tables, and the plan relies on strategy 2 or 3 | the closed set of writers is broken; only constraints and serializable isolation survive foreign writers |
| The rule reads from a replica or a cache | a stale read cannot feed the guard |
| The rule carries a `@within` that the retry bound cannot meet | the budget would be approximated |

**The parity test that shows it.** For every rule with a predicate read and every rule that writes into that predicate (itself included), the harness derives a race test. The interpreter gives the legal observations, one per serial order. On the backend the `store.barrier` handler holds each transaction after its guard reads and releases them together, forcing the bad schedule instead of hoping for it.

```text
race(book, book)  start: slot s with cap 1, no bookings
  interpreter   {A ok, B Full}  or  {B ok, A Full}
  backend       both transactions held after the count, then released
  same          one reply ok, one Full, one row
  diverges      two rows: BACKEND.BUG against the strategy
```

Forced pairs are `bounded(2)` in transactions; larger groups are `sampled`. A backend that cannot script `store.barrier` reports these tests `not transferable`, and the rule's atomicity then rests on the backend token alone.

Open: when serializable retry is exhausted, the caller gets a failure that is not in the rule's meaning. Across places the caller already handles `Unreachable`, and a timer or event is redelivered; inside one place there is no honest typed answer yet.

## What a backend provides for `task`

A `task` spans many commits, calls out, uses replies and resumes. One model, on every backend:

| Term | Is | Key |
|---|---|---|
| **run** | one start of the task, by `@on` or `name.start(...)` | the run key: the `@idempotent` key, else fresh at start |
| **attempt** | one pass over the steps; a new one begins only through `@retry` | (run, attempt number) |
| **step** | each rule call, each outside call, each effect read (`now()`, `Id.fresh()`, `random`), by position | journal entry (run, attempt, position); key given to the outside (run, position) |

A crash inside an attempt resumes that attempt: finished steps answer from the journal, and a step cut off mid-call repeats with the same outside key. A failure named in `@retry(on:)` ends the attempt; after backoff a new attempt runs every step again with the same outside keys, and when the last one fails the `exhausted` rule runs as the final step. A second start with the same run key replays a success; a failed run may start again.

**On a framework whose retry replays stored step results** (durable workflow engines, step-memoising job runners), the framework's retry is the card's crash resume, not the card's attempt.

| Card | Mapped to |
|---|---|
| run | one workflow or job instance, id = run key |
| crash resume | the framework's own replay, left on for worker loss only |
| attempt | a loop the compiler emits inside the instance: catch the typed failure, record "attempt n ended", sleep on a durable timer, continue with n + 1 |
| step memo id | (attempt, position), so a new attempt misses the old results and really runs again |
| outside key | (run, position), unchanged across attempts |

One shortcut is legal: in a new attempt, a step under an `@idempotent` key may be answered from an earlier attempt's stored success, since running it again would replay that reply by definition. Every other step runs again. The difference is observable, so it is tested: another rule changes the state between two attempts, and the observation must show the repeated step's fresh effect.

The backend must also provide: the journal entry in the same commit as the rule a step calls; durable timers for backoff, `@within(over: stop)` and `@on(every(...))`; and for `event`, an outbox row in the emitting rule's commit, delivered to each listener at least once, in that listener's own commit, in the order one rule emitted them. Duplicates are possible; listeners are tested with them.

A backend **must refuse** a `task` when:

| Case | Why |
|---|---|
| The journal cannot commit together with rule state | "finished" could be lost, or recorded for work that rolled back |
| The framework cannot scope stored results by attempt, and the task has a step outside an `@idempotent` key | its retry would replay where the meaning repeats |
| The framework retries typed failures by itself and that cannot be turned off | attempts would be counted twice |
| An unfinished run may meet a new plan and runs cannot be pinned to the plan they started on | positions would name different steps |
| The required crash scope exceeds `survives:` | a kernel thread or a microcontroller loop survives no power loss; it declares `survives: none` and carries only tasks whose policy accepts that |

Once-only effect on a third party stays `assumed` on every backend; none can raise it and none may hide it.

Task parity adds `crash` between any two steps and inside any outside call, and a failure at each step for each attempt. Observations must equal the interpreter's for every such point in a small trace: `bounded(k)` in crash and failure points.

## Refusal

```json
{"code": "E-PLAN-REFUSED",
 "claim": "Speed.within(200us)",
 "at": "motor.slg:12",
 "refused_by": [{"backend": "ultimate", "why": "@refuses(within(under: 1ms))"}],
 "fix": [{"edit": "move Speed to an app with @target(cortex_m4)", "cost": "one more target"},
         {"edit": "drop @within; add a monitor", "evidence": "sampled, monitored"}]}
```

Weakening a claim is a source edit by the author and shows in the guarantee diff. It is never a compiler choice. Among backends that do not refuse, the planner ([10](10-compiler-and-ir.md)) takes the one that carries the kind whole before expanding it.

## Parity: the same tests on every backend

A test's result is an **observation**: replies, final state, and the effect trace projected onto the claims the backend says it keeps. The interpreter produces the expected observation. `spec-lang test --on ultimate` lowers the same tests, runs them on the real toolchain with the backend's scripted handlers (virtual clock, throwaway database), and compares. A trace may start from any world the types, flows and laws allow, so a backend must install a start world directly, not only reach it through rules.

| Outcome | Meaning | Consequence |
|---|---|---|
| `same` | observation equal | counts toward the backend's fidelity |
| `diverges` | observation differs | a backend bug by definition (`BACKEND.BUG`); the test joins that backend's suite; never author work |
| `not transferable` | the backend cannot script a handler the test needs, such as a torn write on a hosted database | listed; lowers fidelity for the claims that test covers; never skipped silently |

The harness takes the user's tests, each vocabulary's suite ([09](09-vocabularies.md)) and terms generated from vocabulary laws.

## Fidelity is evidence on the backend's trust token

Evidence is about the meaning ([04-evidence](04-evidence.md)). A claim carried by a backend adds `backend:ultimate@hash` to its trust set. That token has evidence of its own.

| Fidelity of the token | Earned by |
|---|---|
| `assumed` | a `@carries` line and nothing else; the planner ignores such a line |
| `sampled(suite, n, seed)` | the parity harness passed for this exact vocabulary hash, backend hash and toolchain; recorded as an attestation in `spec-lang.lock` |
| `bounded(k)` | exhaustive parity up to a stated scope, for small finite kinds |

A report line reads: `Note.access: proven, trusts backend:ultimate@31e0 (sampled, n tests, m not transferable)`. Upgrading the framework changes the backend hash, which voids the attestation by construction; `spec-lang test --on ultimate --attest` earns it again.

## Debugging without reading target code

Stack traces are rewritten through `provenance.json` to declarations and strategies. To reproduce, **record at target, replay at spec**: a backend logs effect traces and the trace replays in the interpreter with breakpoints in spec terms. If replay disagrees with production it is a parity bug, and the trace becomes a parity case whose expected outcome is unset until someone approves it.

## Foreign contracts

A `foreign` body is a contract around code the compiler did not produce. Its evidence starts at `assumed` with a `foreign:name@hash` trust token.

```slg
@uses()
@post(result.size == 32)
pub fn sha256(data: Bytes) -> Bytes = foreign c "SHA256";

fn sha256_model(data: Bytes) -> Bytes = sha_rounds(pad(data)).bytes;

law sha_matches_model(data: Bytes) = sha256(data) == sha256_model(data);
```

It can be raised by gating outputs, by a law against a spec-lang model as above (`sampled`), or by a sandbox. Timing, ownership and information-flow claims of unsandboxed foreign code stay `assumed` ([05-effects-and-trust](05-effects-and-trust.md)).

**Hot paths** use the same shape. The slow version is the meaning and stays; the fast one must agree with it.

```slg
fn find_slot(p: Page, key: Bytes) -> Option[u16] = p.keys.position(it >= key);

@within(200ns)
@alloc(none)
fn find_slot_fast(p: ref Page, key: ref Bytes) -> Option[u16] = foreign c "find_slot_simd";

law fast_is_same(p: Page, key: Bytes) = find_slot_fast(ref p, ref key) == find_slot(p, key);
```

## One system, several targets

```slg
@target(go)
app Matchmaker(rules: [join, leave, pair]);

@target(ultimate)
app Admin(rules: [ban, list_players]);
```

| Rule | Mechanism |
|---|---|
| One boundary type | a type crossing two apps gets one canonical wire schema, identified by hash; each backend prints its codec from it; the parity suite round-trips every pair |
| Distribution is visible | a call across places can be `Unreachable`; its reply needs a `task`; atomic means within one place |
| Evolution is checked | against the previous `spec-lang.lock`: a non-additive change without a declared migration is an error naming every app still on the old hash |

## Tiers

[Haxe](https://haxe.org/documentation/introduction/compiler-targets.html) publishes a tier per target and has retired targets. Same here, so nobody learns a backend's standing by surprise.

| Tier | Means |
|---|---|
| 1 | parity attested for this version, the whole suite runs in CI, used by a real app |
| 2 | parity attested, with a listed set of kinds it refuses |
| 3 | experiment: claims on it read `assumed`, and release policy refuses it |
| retired | no new build; existing apps stay pinned to the last attested version |

A backend with no attestation for the current version drops to tier 3 by itself.

## Limits

- **Fidelity is almost always `sampled`.** No suite shows a framework keeps a claim for all programs. Every "proven" on a framework backend is proven of the meaning and sampled in transfer. How good a suite must be is unsolved. This is the weakest point of the architecture.
- **The matrix is sparse.** Attestations cost one suite run per vocabulary, backend and toolchain version; early on, refusal will be common and will feel like low coverage.
- **Atomicity strategies come from a catalogue of guard shapes.** A guard outside the catalogue falls to serializable retry or refusal, and forced races cover pairs, not every schedule a database can produce.
- **Attempt-scoped journals grow** with attempts times steps, and record and replay costs runtime work; both are unmeasured.
