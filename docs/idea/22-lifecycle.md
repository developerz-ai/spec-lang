# Lifecycle

> **Design.** Nothing here runs yet. Adoption into existing code, data change, release, production feedback, upgrades and the way out.

**Rule.** Source is the only thing people and models change. Data, deployed software and incidents are handled as consequences of a source diff or as input to the next test.

## Adopt one declaration at a time

The unit is one action, entity or job inside an existing Ultimate app, never the app.

| Step | What happens | Evidence at the end |
|---|---|---|
| 1 observe | registry facts (entities, actions, policies) become a module contract; bodies stay `foreign` | `assumed`, some monitored |
| 2 characterise | redacted traces are recorded; laws mined from them arrive with origin `import` | `sampled` (recorded traces) |
| 3 intend | a person accepts, rejects or edits each; a bug that traffic always showed must not become a law | same level; origin becomes `person` |
| 4 shadow | the new logic runs on the same inputs in the simulated world; replies and effects are compared, nothing is written twice; each difference is a question | `sampled` (differential) |
| 5 replace | the declaration goes live behind the same typed boundary; the old tests are black-box oracles | its own level; transfer `sampled` for what was exercised |
| 6 count | behaviour that had to be added back is listed | unchanged |

Level and origin are separate facts: how a claim is known, and who meant it ([04](04-evidence.md), [08](08-approval-and-anti-cheating.md)). A mined law can be `sampled` a million times and still be nobody's intent.

```slg
fn may_charge(by: Id[User], order: Id[Order]) -> Bool = foreign ts "billing/policies/mayCharge";

@access(may_charge(by, order) else NotFound)
@uses(http.call, store.write)
@fails(Declined, Unreachable)
@assume("the provider treats one key as one charge")
@idempotent(key)
task charge(by: Id[User], order: Id[Order], key: Text) -> Receipt = foreign ts "billing/actions/charge";

@given(http.reply(503))
test legacy_charge_reports_unreachable = {
  charge(ana, order: Id(1), key: "k1") -> fail Unreachable;
}
```

The old policy arrives as a `foreign` predicate inside `@access`, the only access mechanism, so it is `assumed` until written as logic ([27](27-authorization.md)). Where an old entity is also written by code outside the module, the module does not own it yet: the lifted state is read-only, writes go through `foreign`, and no state law about it can be better than `assumed`. Ownership moves when the last outside writer does.

Callers, tests and laws are written against this now and do not change when the body does. Calls the old code makes around the boundary are listed as uncovered, never counted green. Importing meaning from arbitrary TypeScript is not solved: step 1 reads what a framework already declares, and the logic is drafted by a model and accepted by a person.

## Data changes

A type with stored values has a history. The compiler compares the deployed schema hash with the new one.

| Class | Example | Author owes |
|---|---|---|
| representation | reorder, new index | nothing |
| total | `Int` to a wider range; new field with a default | nothing |
| partial | `Option[Text]` to `Text` | a total repair or quarantine |
| lossy | drop a field | a signed irreversible step |
| contradictory | a law no stored row satisfies | a question; no plan |
| ambiguous | a field gone and a field new: rename, or not? | pick one of two edits |
| due | a flow's `after:` shortened; rows already past it would all move at once | which rows, how fast, or from when |
| moved | a `state` given to another module | nothing if its laws travel; else a question |

```json
{"code":"E-DATA","type":"Order","from":"s:71","to":"s:80","class":"partial",
 "violating":{"count":19,"of":48210,"handle":"q:4"},
 "hole":"repair_order","plan":["expand","backfill","verify","cut","contract"],
 "rollbackUntil":"cut"}
```

The repair is ordinary logic with ordinary tests. The old shape is a generated read-only module, `before` (a proposal: the card has no form for it yet):

```slg
@why("nobody guesses an address; those orders are set aside for a person")
@fails(Quarantine)
fn repair_order(o: before.Order) -> Order = {
  let email = o.email else fail Quarantine;
  return Order(o.id, email: email, total: o.total);
}

test repair_keeps_money(o: before.Order where self.email != none) = {
  repair_order(o).total -> o.total;
}
```

Obligations, checked like any other: every old value maps or is quarantined; the backfill resumes after a crash at any point and is idempotent; laws hold on the result; if old and new versions overlap, traces mix them. If overlap cannot hold a law, the plan says downtime. The transition is generated and deleted after `contract`; no compatibility code stays in source. A business mapping is never inferred from a schema diff.

**Runs in flight.** A `task` run may outlive a release, and its finished steps were steps of the old body. A run finishes on the body it started with, pinned by hash, and new runs use the new one. The state between steps is shared, so the laws are explored with both bodies in one trace; if that refutes a law, the plan drains old runs first. Events wait the same way: an event emitted before the release must still decode after it, so an event type changes under the same classes as stored data.

## Release

`build` is a function of source, lock file, compiler, vocabularies and targets. Same inputs, same bytes. `land` writes an attestation outside source: authenticated principal, model and session when supplied (marked self-reported otherwise), base and head, approval, obligations met, hashes of inputs and outputs. No prompts and no reasoning text are stored.

Generated software is never edited; a tree that differs from its attestation fails the gate. An error from a target's own compiler is `BACKEND.BUG`: a defect in the backend, with a minimal source reproduction, never work for the author.

## CI and delivery

Cheap truth first. Each stage runs only if the one before is green, and only for declarations whose hash changed.

| Stage | Runs | Red means |
|---|---|---|
| 1. Logic | types, tests, laws in the interpreter; nothing is built | the logic is wrong |
| 2. Guarantees | the guarantee diff against the base commit's policy | something got weaker and nobody approved it |
| 3. Generate | the compiler emits only changed targets; same source, same bytes | the plan refused, or a backend cannot keep a guarantee |
| 4. Build | the target toolchains compile what changed | a compiler or backend bug, not author work |
| 5. Parity | the **same tests** on the built software, plus what the model cannot see: real database, real device, load, screens | the backend diverged from the meaning |
| 6. Deliver | migration plan, rollout, monitors for claims that are only monitored | a claim failed in production: a candidate test |

A red stage names a kind of fault, so an agent knows where to look. An edit that changes only comments, local names or formatting stops after stage 1. An edit to one rule rebuilds that rule's targets, so a game with one changed rule does not need a full build to learn the rule is wrong.

**Locally: is the logic right? In CI: does the real thing agree?**

| | Local, every edit | CI, every landing |
|---|---|---|
| Question | is the logic correct | does the built software do the same |
| Needs | the interpreter | databases, devices, browsers, target compilers |
| Runs | affected tests, laws, guarantee diff | generate, compile, parity, load, screens |
| Who waits | the agent, briefly | nobody: the agent goes on to other work |
| A failure is | author work | a backend, a simulator, or the world |

What makes the split safe is that a CI failure comes back as something local. The real run's effects are recorded and become a replay: the failing input, what the simulator answered, what the real system answered. Three results follow:

| The replay shows | Then |
|---|---|
| the backend did something the meaning forbids | backend bug; the author's logic stands |
| the simulator was wrong about the world | the handler is corrected, so every later local run knows it |
| the logic assumed something false | a candidate test, red locally, fixed in the fast loop |

So each surprise is paid for once, and the local world gets closer to the real one. An agent may run one parity test locally when it wants to; it is not the default.

Stage 1 never replaces stage 5. Green logic with a wrong backend is a wrong program.

## Production feeds the next test

```text
claim marked monitored -> sampled check in production -> violation
  -> bounded, redacted replay -> shrink in the interpreter -> candidate test with an unset expectation
  -> a person or the law's owner sets it -> runs on every backend, for ever
```

A candidate arrives as source, blocked by a hole:

```slg
@why("incident: the reply was lost and the card was charged again")
@given(clock.at(0s), http.reply(200), net.drop(1))
test lost_reply_then_retry = {
  charge(ana, order: Id(1), key: "k1") -> ?expected_first;
  charge(ana, order: Id(1), key: "k1") -> ?expected_second;
}
```

Production shows what happened, not what should have. A hole in an expectation makes that a type-level fact: the test is `Blocked` until someone with authority decides.

| Concern | Handling |
|---|---|
| private data | `@pii` and `@secret` values are replaced while shrinking, keeping the path; if the path needs the value, it stays in the protected environment behind a handle |
| hostile data | the replay is typed values; its text is untrusted, never placed in a slice ([07](07-agent-loop.md)) |
| nondeterminism in the target | the replay may not reproduce: parity result `diverges`, a backend lead |
| a claim reported `proven` fails | if the replay reproduces in the interpreter, the claim is `refuted`: the prover or a law was wrong. If not, a token it trusts is refuted (backend, store, hardware, `@assume`): the claim is reported `stated` until that token is repaired, and release is blocked |
| no violation seen | the claim stays at its level; `monitored` never raises it |

Debugging stays in source: stack frames, logs and profiles map back to declarations, and a recorded effect trace replays in the interpreter. Opening generated code is reading disassembly, allowed and a sign of a suspected backend bug ([10](10-compiler-and-ir.md)).

## Upgrades

| What moved | Treated as |
|---|---|
| a package or vocabulary | a guarantee diff; its proofs are re-checked, not trusted |
| a default policy | a listed behaviour change, even with no source edit |
| the compiler or solver | evidence that named the old one is re-earned; a claim the new one cannot decide drops to the level still earned, shown in the diff |
| a framework version | its backend re-runs conformance; attestation is per version |
| the card | printed by `view card`, versioned with the compiler; old source is rewritten by a formatter step that preserves IR, or the change is a break |

## Three budgets, kept apart

| Budget | Declared on | When exceeded |
|---|---|---|
| runtime: `@within`, `@alloc`, size | declaration | checked where it can be, else monitored; the default only reports |
| verification: solver steps | obligation | the claim stays at the level already earned, reproducibly |
| context: `editCost` | declaration, tracked | a report that this declaration has become expensive to change |

Agents have no work budget. A stall is handled by [07](07-agent-loop.md).

## The way out

`edit --eject` can eject one declaration: its generated code becomes hand-owned and the declaration's body becomes `foreign`. Types, contract and tests remain and keep running; claims resting on it drop to `sampled` where tests cover them, else `assumed`. It is one-way, per declaration, and the reason is recorded.

If the language itself is stopped ([19](19-density-and-experiments.md), K3), every module can be ejected, or its IR printed as the embedded TypeScript form. That printer is part of the E0 work, so the exit exists before there is anything to leave.

## Limits

- **Accumulated edge cases.** Some plumbing is hard-won behaviour. Shadowing finds only what traffic exercised.
- Shadowing effects needs the simulated world to behave like the real one.
- Expand and contract multiplies states during a rollout; mixed-version exploration may only reach small bounds.
- Redaction that preserves a failing path is not always possible.
- Pinning runs keeps old bodies, and their bugs, alive for as long as the longest run.
- `before` needs a decision in the card, and long-lived data may need more than one past version at once.
- Ejected code is generated code; nobody has yet had to live with it.
