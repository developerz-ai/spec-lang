# Backends and parity

> **Design.** Nothing here runs yet. What a backend must declare, how the same tests judge every backend, when a backend must refuse, and how target languages are treated.

**Rule: target languages are the compiler's intermediate representation.** TypeScript, Rust, C, C++, Swift, Kotlin, Go, SQL, WGSL and deployment manifests stand to spec-lang as assembly stands to C. Frameworks are higher-level targets of the same kind ([12-frameworks](12-frameworks.md)). Nobody authors them by default, nobody edits the output, nobody approves it as truth.

**Rule: a backend that cannot keep a claim refuses.** It never approximates.

## Who touches target code

| Who | May | Why |
|---|---|---|
| Application author | never edit generated files; the build overwrites them and the gate rejects a tree whose hash differs from the plan's | output is not source |
| Application author | write a `foreign` body | trusted code already exists, the thing is below every altitude, or a measured hot path beats every strategy |
| Backend author | write printers and runtime shims in the target language | that is the backend |
| Anyone debugging | read generated code, as one reads disassembly | allowed; it signals a suspected backend bug, and the answer is a parity case, never a patch |

Generated code is still deterministic, stably named and lint-clean, for toolchains and rare inspection, not for review.

## The backend contract

A backend is a package that `use`s the `compiler` vocabulary and declares what it carries.

```slg
use compiler;

@toolchain(bun)
@carries(data.entity, keeps: [identity, access, tenant, durable])
@carries(data.query, keeps: [access, page, order])
@carries(sync.live, keeps: [order_per_entity])
@carries(task, keeps: [step_once, resume, retry, idempotent], survives: process)
@refuses(within(under: 1ms), alloc, exactly_once)
@handlers(store, clock, http, job)
@scripts(clock, store, http)
@hosts(typescript)
backend ultimate;
```

| Note | Declares | Planner effect |
|---|---|---|
| `@carries(kind, keeps: [...])` | a vocabulary kind it takes whole, and the claims it keeps | a claim not listed is not kept; the node is infeasible there if policy needs it |
| `@refuses(...)` | claims it can never keep; an argument narrows the refusal, so a `@within` deadline of seconds on a `task` is carried as a timer while a sub-millisecond budget is refused | any declaration carrying one cannot be placed on it |
| `@handlers(...)` | effects it can implement | a rule using another effect cannot be placed on it |
| `@scripts(...)` | handlers it can drive deterministically in tests | decides which tests transfer |
| `@hosts(lang)` | languages it accepts in `foreign` bodies | checks `foreign` placement |
| `@toolchain(...)` | the pinned external tools | part of the backend hash |

## What a backend provides for `task`

A `rule` is atomic and never waits on the outside world: a transaction and an after-commit outbox carry it. A `task` spans many commits, calls out, uses replies and resumes. The compiler cuts it into steps (each rule call, each outside call) and keys every step by task identity, arguments and position. The backend supplies the rest.

| The backend must provide | Because the card says |
|---|---|
| A durable **journal**: step key, status, recorded reply | a step that finished is not run again |
| Journal write in the same commit as the rule a step calls | each rule the task calls commits alone, and finishing is not lost |
| **Resume**: after a restart, re-enter every unfinished task at its first unfinished step, with recorded replies replayed | a task resumes after a crash |
| The step key passed to the outside as an idempotency key | a step cut off mid-call is retried with the same key |
| Timers for `@retry` backoff, `@within(over: stop)` and `@on(every(...))` that survive the same crashes | policies outlive the process |
| An `event` log with at-least-once delivery to every `@on` listener | an emitted event is a fact; listeners are tasks or rules |

On Ultimate this maps to its job primitive plus a journal table ([12](12-frameworks.md)); a workflow engine carries the whole task natively, and the planner prefers that.

A backend **must refuse** a `task` when:

| Case | Why |
|---|---|
| It has no durable store, or the journal cannot commit together with rule state | "finished" could be lost or recorded for work that rolled back |
| The task's code version may change under an unfinished run and the backend cannot pin runs to the plan they started on | step keys would no longer name the same steps |
| The required crash scope exceeds `survives:` | a kernel thread or a microcontroller loop survives no power loss; such a backend declares `survives: none` and carries only tasks whose policy accepts that |
| An outside call has no idempotency key the other side honours and the task claims once-only effects | the claim stays `assumed` on every backend; a backend cannot raise it and must not hide it |

Parity for tasks adds one handler to every scenario: `crash` between any two steps and inside any outside call. The observation must equal the interpreter's for every crash point in a small trace, so task parity is `bounded(k)` in crash points even when everything else is `sampled`.

## Refusal

```json
{"code": "E-PLAN-REFUSED",
 "claim": "Speed.within(200us)",
 "at": "motor.slg:12",
 "refused_by": [{"backend": "ultimate", "why": "@refuses(within(under: 1ms))"}],
 "fix": [{"edit": "@target(cortex_m4) on Speed", "cost": "one more target"},
         {"edit": "drop @within; add a monitor", "evidence": "sampled, monitored"}]}
```

Weakening a claim is a source edit by the author and shows in the guarantee diff. It is never a compiler choice.

## Lower to the highest target that keeps the claim

The planner ([10](10-compiler-and-ir.md)) prefers a backend that carries a kind whole over an expansion into lower kinds, and an expansion over raw code.

```text
entity Note + @access + @page
  1. ultimate carries data.entity, keeps access and page    -> one entity, one policy, generated by the framework
  2. else expand to state + rules + laws; a web framework carries rules
  3. else expand to core; a language backend emits tables, handlers and checks
```

Each step down emits more code and moves claims from "kept by a framework trusted by name" to "kept by code this compiler printed". The ledger shows which applies.

## Parity: the same tests on every backend

A test's result is an **observation**: replies, final state, and the effect trace projected onto the claims the backend says it keeps. The interpreter produces the expected observation. `spec-lang test --on ultimate` lowers the same tests, runs them on the real toolchain with the backend's scripted handlers (virtual clock, throwaway database), and compares.

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
| `proven` | a verified translation; not expected for any framework backend |

A report line reads: `Note.access: proven, trusts backend:ultimate@31e0 (sampled, n tests, m not transferable)`. Upgrading the framework changes the backend hash, which voids the attestation by construction; `spec-lang test --on ultimate --attest` earns it again.

## Debugging without reading target code

| Need | Mechanism |
|---|---|
| A stack trace in production | rewritten through `provenance.json` to declarations and strategies |
| Reproduce | **record at target, replay at spec**: a backend logs effect traces; the trace replays in the interpreter with breakpoints in spec terms |
| Replay disagrees with production | a parity bug; the trace becomes a parity case with its expected outcome unset until someone approves it |

## Foreign contracts

A `foreign` body is a contract around code the compiler did not produce. Its evidence starts at `assumed` with a `foreign:name@hash` trust token.

```slg
@uses()
@post(result.size == 32)
pub fn sha256(data: Bytes) -> Bytes = foreign c "SHA256";

fn sha256_model(data: Bytes) -> Bytes = sha_rounds(pad(data)).bytes;

law sha_matches_model(data: Bytes) = sha256(data) == sha256_model(data);
```

Three upgrades exist: gate the outputs (value claims then hold for every value that gets through), a law against a spec-lang model as above (`sampled`), and a sandbox (effect claims hold given the sandbox token). Timing, ownership and information-flow claims of unsandboxed foreign code stay `assumed` ([05-effects-and-trust](05-effects-and-trust.md)).

**Hot paths** use the same shape. The slow version is the meaning and stays; the fast one must agree with it.

```slg
fn find_slot(p: Page, key: Bytes) -> Option[u16] = p.keys.position(it >= key);

@within(200ns)
@alloc(none)
fn find_slot_fast(p: ref Page, key: ref Bytes) -> Option[u16] = machine {
  var lo = 0u16;
  var hi = p.nslots;
  while lo < hi {
    let mid = lo + ((hi - lo) shr 1);
    if p.key_at(mid) < key { lo = mid + 1; } else { hi = mid; }
  }
  return if lo < p.nslots then Some(lo) else none;
}

law fast_is_same(p: Page, key: Bytes) = find_slot_fast(ref p, ref key) == find_slot(p, key);
```

A time budget is `sampled` by a committed benchmark and can be monitored. No static performance claim is made.

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
| Distribution is visible | a call crossing apps gains a network effect, and the caller must handle `Unreachable`, `Timeout` and `VersionSkew` |
| Evolution is checked | against the previous `spec-lang.lock`: a non-additive change without a declared migration is an error naming every app still on the old hash |

## Leaving

`spec-lang edit --eject Note` rewrites the declaration's body to `foreign`, hands the last generated files to the author, keeps the derived contract and tests, and resets evidence to what those tests justify. It is one-way and a guarantee diff: insurance against lock-in, not a way of working.

## Limits

- **Fidelity is almost always `sampled`.** No suite shows a framework keeps a claim for all programs. Every "proven" on a framework backend is proven of the meaning and sampled in transfer. How good a suite must be is unsolved. This is the weakest point of the architecture.
- **Unlisted behaviour is not portable.** Two backends carrying one kind will differ in ordering, error text and timing. Authors will come to depend on the differences.
- **Simulated handlers are models.** A real database deviates from the in-memory one exactly where no parity test looks.
- **The matrix is sparse.** Attestations cost one suite run per vocabulary, backend and toolchain version. Early on, refusal will be common. That is correct behaviour and will feel like low coverage.
- **Record and replay, and task journals, cost runtime work** on every backend; that cost is unmeasured.
