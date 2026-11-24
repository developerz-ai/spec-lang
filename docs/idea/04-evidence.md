# Evidence

> **Design.** Nothing here runs yet. This doc defines how spec-lang records the way each claim is known, how that composes across calls, foreign code and backends, and what the compiler does when it cannot decide.

**Rule: every claim reports how it is known, and the compiler never rounds up.** What a claim is: [03-types-and-claims](03-types-and-claims.md).

## The gap, stated first

Evidence is about the **meaning** of the source: what the interpreter and the verifier see. The software that ships is generated code running on a framework, a database and hardware. Whether that code keeps the claim is a separate question with its own, usually weaker, evidence. Two independent drafts of this design reached that conclusion, so it is built into every report line:

```text
conserved          proven; trusts backend:ultimate (transfer: sampled), store:sql.atomic (assumed)
wx                 proven; trusts backend:c (transfer: sampled), hw:x86_64.tlb (assumed)
frame_time         sampled, monitored; trusts bench:scenes@4f1a (sampled), backend:anygame (transfer: assumed)
```

No line ever reads just "proven". The words after the semicolon are part of the guarantee.

## The scale

Evidence is a triple.

```text
evidence = static × monitored × trust

static    : proven > bounded(k) > sampled > assumed > stated      (refuted is an error, not a level)
monitored : yes > no
trust     : a set of named axioms, each with its own static level; fewer and stronger is better

meet(e1, e2) = (min static, min monitored, union of trust)
```

| Level | Means | Produced by |
|---|---|---|
| `proven` | holds for all inputs; replayable certificate | structural checkers, solver |
| `bounded(k)` | exhaustive up to a stated scope: trace depth, collection size, crash points | bounded search |
| `sampled` | held on every case tried; records generator, count, seed | examples, properties, simulation, benchmarks |
| `assumed` | admitted on purpose, with a reason; listed | the author |
| `stated` | written, not yet run, or blocked by a hole | default |

`bounded` levels compare only for the same scope shape; `sampled` levels from different generators do not compare. The report prints both verbatim.

**Trust tokens** are content-addressed names for things outside the proof: `solver:smt@h`, `backend:ultimate@h`, `store:sql.unique`, `hw:disk.fsync`, `foreign:sha256@h`, `eval:sql_cases@h`, `model:m@v`. Each token carries a static level of its own. For a `backend:` token that level is called **transfer**: how it is known that the emitted code refines the meaning for the constructs this claim uses.

| Transfer | Means |
|---|---|
| `proven` | translation validated for these constructs |
| `sampled` | the same tests gave the same observations on the interpreter and on this backend ([11-backends-and-parity](11-backends-and-parity.md)) |
| `assumed` | no parity test can exercise it on this backend |

**Monitored** is a separate axis because it answers a different question: if the claim breaks in production, will anyone know? It never raises the static level.

## A guard is a proof rule, not a level

`@guard(p else E)` checks `p` at run time. That does not make `p` "runtime evidence". After the guard, `p` is `proven` for every execution that continues, and `E` is in the declaration's failures. The uncertainty moved into the interface, where callers must handle it. The same holds for `x else fail E`, a `match` arm, and decoding at a trigger. What is not claimed is that the rule succeeds.

## How a claim is checked

Each claim is assigned a tier from its syntax alone, before anything is solved, so the author knows the cost of what was written.

| Tier | Mechanism | Covers | Verb |
|---|---|---|---|
| T0 | structural checkers; always terminate | types, units, labels, effect rows, ownership, layout, flow edges | every edit |
| T1 | solver, decidable theories | linear arithmetic, equality, bit-vectors, finite collections, sums and records | `prove` |
| T2 | solver, semi-decidable | unbounded quantifiers, nonlinear, text, heap shapes | `prove` |
| T3 | bounded search | trace laws, crash points, small scopes | `prove` |
| T4 | generated tests and deterministic simulation in the interpreter | anything executable | `test` |
| T5 | monitor in the generated software | executable claims, budgets | `build` |
| T6 | admission | anything | the author |

Rules for the undecided case:

1. **Refuted is always an error**, with the smallest counterexample found. Nothing overrides it.
2. **Undecided descends in a fixed order**: T1, T2, T3, T4. The claim keeps the best level reached.
3. **Solver budgets are step counts, not seconds**, with a pinned solver, so "undecided" is the same on every machine. Otherwise builds are not replayable.
4. **Below policy is an error that names the fixes**: strengthen a `@pre`, add a `@guard`, supply a lemma as another `law`, lower the policy, or admit.
5. **Every `@pre` needs a witness** (a test case or a solver model). A precondition nothing satisfies proves anything.

## One claim, no rewrite

A claim is written once. It is a test oracle, a generator description, a proof obligation and a monitor at the same time. Only which of these has run changes.

```slg
use storage;

type Record = Put(tx: TxId, key: Bytes, value: Bytes) | Commit(tx: TxId);
state wal: Log[Record];
state acked: Set[TxId];

@uses(wal.write)
rule put(tx: TxId, key: Bytes, value: Bytes) = { wal.append(Put(tx, key, value)); }

@why("an acknowledged commit survives power loss")
@uses(wal.write, disk.sync, acked.write)
@fails(DiskError)
pub rule commit(tx: TxId) = {
  let lsn = wal.append(Commit(tx));
  try wal.sync(lsn);
  acked.insert(tx);
}

fn recover(disk: List[Record]) -> List[Record] = ?recover;

law durable(t: Trace[put | commit | crash]) =
  t.states.all(s => s.acked.all(tx => recover(s.wal.disk).contains(Commit(tx))));

law atomic(t: Trace[put | commit | crash], tx: TxId) =
  t.states.all(s => applied(recover(s.wal.disk), tx) in [Nothing, Whole]);

law recover_once(d: List[Record]) = recover(recover(d)) == recover(d);

@given(disk.sim(torn_writes: true))
test ack_survives_crash = {
  put(Id(1), key: b"k", value: b"v");
  commit(Id(1));
  disk.crash();
  assert recover(wal.disk).contains(Commit(Id(1)));
}
```

`crash` is an event supplied by the disk handler: it drops everything after the last `sync` and may tear the final write. The life of `durable`:

| Moment | What ran | Ledger |
|---|---|---|
| `recover` is still a hole | T0 | `stated`; tests that reach the hole report `Blocked`, the rest run |
| first `test` | the law is evaluated on every state of every test trace | `sampled(examples: 1)` |
| same `test`, a moment later | traces generated from `Trace[put \| commit \| crash]`, shrunk on failure | `sampled(traces: n, seed)` |
| `prove` | every interleaving with a crash at each effect boundary, up to `k` events | `bounded(k)` |
| `prove`, if the law is inductive | solver | `proven` |
| body rewritten as `machine { ... }` with real file descriptors | same law, same tests | back to `bounded(k)`: heap reasoning is T2 |
| `build` for a target | parity tests on the backend | `...; trusts backend:c (transfer: sampled), hw:disk.fsync (assumed)` |

| Profile | Blocks on | For |
|---|---|---|
| `sketch` | T0 errors and refutations | the loop: idea, logic, test, counterexample, fix |
| `release` | the project policy, and any remaining hole | `land` |

Type errors always block, because they are instant and are the cheapest counterexamples there are. Proof never blocks a sketch. A shrunk counterexample is kept as a stored case, so a failure found once is replayed forever. An example is a claim with its variables bound; a property is the claim with them quantified; a proof is the quantifier discharged.

## Composition

| Across | Rule |
|---|---|
| derivation | a derived claim has the meet of its premises |
| a call | `evidence(@post at the call) = meet(callee's @post, discharge of its @pre)`; read from the callee's interface hash, the body is not re-verified |
| a vocabulary kind | the kind's laws are checked once on its expansion; users inherit them with the vocabulary's token in the trust set |
| a backend | adds `backend:X` with its transfer level |
| foreign code | starts at `assumed`, with `foreign:name@h` in the trust set |

Foreign code, including hand-written code around one spec module in an existing app, can be raised in three ways, each limited.

| Upgrade | Works for | Result |
|---|---|---|
| guard its outputs at the boundary | executable value claims | `proven`, plus a failure in the interface |
| differential test against a `fn` that defines the meaning | value and trace claims | `sampled` |
| sandbox the call | effect claims | `proven`, trusting the sandbox |

Ownership, timing and information-flow claims of unsandboxed foreign code stay `assumed`.

## Policy, admission, ledger

Policy is project configuration, not source. Admission is source: the card's `@assume("reason")` on the claim's declaration.

```text
policy release
  law.*              >= proven    transfer >= sampled
  law in ui.*        >= sampled
  @within            >= sampled, monitored
  @tenant @secret    >= proven    transfer >= sampled
  assumed            allowed only with @assume and a named approver
```

The achieved evidence of every claim is written to a committed, content-hashed ledger. Three consequences:

- **A drop is a diff.** Evidence for an unchanged claim cannot fall without a visible ledger change, which is what a human approves ([08-approval-and-anti-cheating](08-approval-and-anti-cheating.md)).
- **Invalidation is exact.** A new solver, backend, model or eval set invalidates the entries whose trust set names it, and no others.
- **Policy reads the leaves.** The ledger stores the derivation's leaves, not only the summary triple.

## Evals are evidence for a different claim

Running a model on 400 cases says nothing certain about case 401. It can support a claim about a rate.

```slg
use ai;

law mostly_right = evals.sql_cases.pass_rate(answer) >= 90%;
```

Such a law is a distribution claim. Its best possible level is `sampled`, with `eval:sql_cases@h`, `model:m@v` and the assumption that the eval set resembles production all in the trust set. Evals run outside the build and enter as hashed ledger entries; changing the model or the prompt invalidates them. A distribution claim never discharges a value claim: to use one output as `T where p`, guard it ([05-effects-and-trust](05-effects-and-trust.md)).

## Limits

- **Transfer is `sampled` almost everywhere.** A parity suite cannot show that a framework preserves a guarantee for all programs. Validated translation per backend is the only real fix and is not scoped. This is the weakest point of the design.
- **The meet is a label, not a theorem.** "A is proven given P, and P is bounded(8)" is not "A is checked to 8". The ledger keeps the leaves and reports `proven given {P: bounded(8)}`; a single summary word would overstate it.
- **Admitted claims can contradict each other** and then prove anything. Joint satisfiability is checked at T1 only.
- **Bodies at machine altitude will mostly sit at `bounded`.** The write-ahead log above shows the normal outcome.
- **Gradual evidence invites permanent sketches.** Only the release policy pushes claims upward. A project that sets every policy line to `sampled` has a well-tested language, not a verified one.
