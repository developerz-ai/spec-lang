# Effects and trust

> **Design.** Nothing here runs yet. This doc defines effects, capabilities, errors, budgets, information-flow labels, the trust boundary for outside data and model output, and determinism per backend.

**Rule: a declaration can do only what its `@uses` says, see only what its labels allow, and believe only what a guard has checked.** Claims in general: [03-types-and-claims](03-types-and-claims.md). How each is known: [04-evidence](04-evidence.md).

## Effects and capabilities

| Concept | Is | Written |
|---|---|---|
| effect | a named operation on something outside the computation: `seats.write`, `clock.read`, `http.call`, `llm.call` | `@uses(...)`; inferred when private, required on `pub` |
| capability | the authority to perform an effect on a specific thing | never written; held by the `app`, narrowed on the way down |
| handler | an implementation of an effect | `handler`; the simulated ones are the default in `test` |
| failure | a typed refusal | `@guard(p else E)`, `@fails(E)`, `fail E` |
| reply | what the outside answered | only in a `task`; always untrusted |

1. **Effects are a row, checked structurally.** A `fn` has the empty row. A body that performs an effect missing from a `pub` declaration's `@uses` is an error, at once.
2. **An effect is the static shadow of a capability.** `@uses(Invoice.read)` is legal only where the authority to read invoices is in scope. Authority is minted in one place, the `app`, so the set an app mints **is** its permission manifest, its access policy and its network policy.
3. **Capabilities narrow, never widen.** Inside a rule exposed with `by`, store access is already restricted to what `@access` and `@tenant` allow for that caller. Absent `@access` means denied.
4. **Handlers make every program simulable.** Clock, network, disk, store and model are effects; a test swaps their handlers.
5. **Errors are part of the interface.** No exceptions and no panics. A partial operation (`m[k]`, `a.div(b, floor)`) carries a claim; if it is not discharged the author picks `@pre`, `@guard` or `else`. The compiler never inserts a hidden check.
6. **Absence is a fact.** `clock.read`, `random`, `alloc` and `float.native` are effects. A row without them is deterministic, allocation-free or bit-exact by a structural check.

## Budgets

A budget is a grade on the computation, drawn from an ordered monoid: sequence adds, branches take the larger, loops multiply by a bound.

| Budget | Form | Enforcement |
|---|---|---|
| `@alloc(none)` | absence of an effect | structural; `proven` |
| `@within(d)` | static cost claim | depends entirely on the target; see below |
| tokens, money | **linear**: the budget is a value that calls split and consume, because a call's maximum cost is known | exact at run time; overspend is a typed failure before the call, never after |

```slg
use game;

@on(tick(60Hz))
@within(16ms)
@alloc(none)
rule frame(dt: Time) = {
  poll_input();
  simulate(dt);
  cull();
  submit();
}

@within(4ms, per: 10_000)
@alloc(none)
rule simulate(dt: Time) = ?simulate;
```

| Claim | Evidence | Reason |
|---|---|---|
| no allocation in `frame` | `proven` | `alloc` is not in the row of anything it calls |
| stage budgets fit in 16 ms | `proven` given the leaves | arithmetic over declared `@within` |
| a leaf's `@within` on a microcontroller with a timing model | `proven`, trusting the model | static worst-case analysis |
| the same on a console, a garbage-collected runtime, or a GPU | `sampled, monitored` at best | a benchmark over named scenes, plus a runtime monitor |

On overrun the default only reports; `over: stop` or `over: refuse` acts. A gentler action, such as shedding detail, would be a `game` vocabulary policy checked against the same budget. The ledger line reads `sampled, monitored; trusts bench:scenes (sampled)`. It never reads "guaranteed 60 fps".

## Confidentiality labels

`@pii`, `@secret` and `@tenant(x)` are lattice grades. Every operation joins the labels of its inputs, including the condition of any branch it sits under. Every exit (a reply, a log line, an outgoing request, a model prompt) has a maximum label. The check is structural, decidable and conservative.

```slg
use web;

@tenant(org)
@access(member -> read, admin -> all)
entity Invoice(id: Id[Invoice], org: Id[Org], total: Money[USD], @pii memo: Text[0..=500]);

@uses(Invoice.read)
pub query unpaid(by: Id[User]) -> List[Invoice] = Invoice.filter(it.total > 0USD).sort(it.id);

fn seen(t: Trace[bill | unpaid], o: Id[Org]) -> List[Step] =
  t.steps.filter(it.before.users[it.event.by].org == o);

law isolated(a: Trace[bill | unpaid], b: Trace[bill | unpaid], o: Id[Org]) =
  seen(a, o).map(it.event) == seen(b, o).map(it.event)
    implies seen(a, o).map(it.reply) == seen(b, o).map(it.reply);
```

`isolated` is a run-pair claim: two histories that look the same to one organisation give it the same replies, whatever the others did. It is what `@tenant(org)` means; nobody writes it by hand, and it is not established by searching pairs of traces.

| Step | Mechanism |
|---|---|
| a row of `Invoice` has label `tenant(row.org)` | from the note |
| the whole table has the join of all tenants | lattice |
| `unpaid` is exposed with `by`, so the store capability is narrowed to the caller's organisation | derived scoping; the author did not write the filter |
| the result has that organisation's label, which the reply to `by` accepts | equality reasoning on ids |
| a query that sums across organisations has the top label | no reply accepts it without an explicit, listed declassification |

Row-level policy and log redaction for `@pii` are derived from the same labels. Expected evidence: `proven; trusts backend:ultimate (transfer: sampled), store:sql.row_policy (assumed)`.

## Integrity: outside data and model output

**Everything that enters is untrusted, by rule, with no note.** A request body, a file, an environment variable, a foreign return value and a model's output all start with nothing believed about them except their bytes.

| Step | What it establishes |
|---|---|
| decoding at a trigger | the parameter types of an exposed rule are checked before guards run; a declared external type is not evidence, the decode is |
| `@guard`, `else fail`, `match` | one claim each, `proven` for the code after it |
| reaching a sink | legal exactly when every `@pre` of the sink is `proven` on the value; there is no cast that says "trust this" |

"Trusted" is not a bit. Integrity is lowered per claim, by a check, or not at all.

**Replies exist only in a `task`, and every reply is untrusted.** A `rule` is atomic and never waits on the outside: what it sends leaves after commit and nothing comes back. So outside data reaches logic by exactly two doors, a trigger's parameters and a reply inside a `task`.

| Fact about a reply | Rule |
|---|---|
| its label | the callee's source (`http:host`, `model:m@v`) joined with the labels of everything sent in the call |
| what it proves | nothing but its decoded shape; a declared reply type is checked at the call, and a mismatch is the call's failure |
| across a crash | a finished step is not run again, so its reply is journalled; the label is journalled with it, and resuming never launders it |
| a retried step | the retry may return a different reply; claims established on the first attempt do not carry over |
| passing it to a `rule` | each rule the task calls commits alone and its `@pre` must be `proven` on the argument; guards sit in the task or in the rule |
| confidentiality the other way | the call is an exit: a `@pii` or `@tenant` value in a prompt or a request body needs a handler that accepts that label |

**Model output inherits the integrity of everything in its prompt.** If retrieved web text is in the context, the output is as untrusted as that text. Prompt injection then stops being a special case: a value influenced by outside text cannot choose a tool, a shell argument, a write or a payment unless it passes the guards that sink declares.

```slg
use ai;
use sql;

@pre(q.tables.all(it in views(org)), q.estimated_rows <= 100_000)
@uses(db.read)
rule run(org: Id[Org], q: Select) -> List[Row] = db.query(q);

@uses(llm.call, db.read)
@fails(Rejected)
@within(8s)
@spend(tokens: 4000, cost: 0.02USD)
@access(user -> all)
pub task answer(by: Id[User], q: Text) -> List[Row] = {
  let org = org_of(by);
  let raw = llm.call("One read-only SELECT answering: {q}", context: schema_of(org)) else fail Rejected;
  let ast = parse[Select](raw) else fail Rejected;
  if not ast.tables.all(it in views(org)) { fail Rejected; }
  if ast.estimated_rows > 100_000 { fail Rejected; }
  return run(org, ast);
}

law mostly_right = evals.sql_cases.pass_rate(answer) >= 90%;
```

| Claim about the model's text | Fate | Evidence |
|---|---|---|
| it is one well-formed `SELECT` | checked by `parse` | `proven` after the line; `Rejected` otherwise |
| it touches only this organisation's views and is bounded in cost | checked by the two `if` lines, which discharge `run`'s `@pre` | `proven` |
| it answers the question | cannot be checked on one value | a distribution claim: `sampled; trusts eval:sql_cases, model:m@v` |

Delete either `if` and the call to `run` does not compile. The store capability is narrowed to the caller's organisation as well, so the damage is bounded even if a guard is wrong. `@spend` is a note the `ai` vocabulary would supply; it is the linear budget above.

## Determinism and numbers, per backend

Determinism is a run-pair claim: equal inputs and equal handler replies give equal outputs. Each backend declares which claims it preserves. Lowering a declaration to a backend that cannot preserve one of its claims is an error, never a silent downgrade.

| Value | Meaning | Across backends |
|---|---|---|
| `Int` | exact, unbounded | identical; width chosen from value claims |
| `u8 ... i64` | wrapping, by definition | identical |
| `Float`: `+ - × ÷`, square root, fused multiply-add, conversions | correctly rounded, no reordering, no contraction, one NaN | bit-identical on conforming targets |
| `Float`: `sin`, `exp` and the rest | either a software version (bit-exact, slower) or the effect `float.native` | with `float.native` the claim weakens to an error bound and determinism is lost |
| `Float` on a GPU backend | approximate only | exact code is rejected there; `x ± e` types state the bound |

## Concurrency and distribution

- **Above machine altitude there is no shared mutable state.** A rule is atomic; traces explore every interleaving the triggers allow. A `task` is not atomic: other rules run between its steps, so anything it read before a call is stale after it and must be re-checked by the next rule's guards.
- **At machine altitude** data-race freedom follows from `own`, `ref` and `mut`; deadlock freedom only where locks carry an order.
- **Replicated state** carries a consistency label that joins like any other, so a decision made on an eventual read cannot support a claim that needs a linearizable one.
- **Coordination is derived from the laws.** For each trace law over replicated state the compiler asks whether merging two law-abiding divergent histories keeps the law. If yes, the rule runs without coordination. If no, the counterexample is two concrete concurrent calls, and the rule needs coordination. "Balance never negative" under concurrent `transfer` is the standard failing case.

## Limits

- **Labels ignore side channels.** Timing, response size, cache behaviour and anything inside foreign code are outside the label check. The isolation claim covers explicit and implicit flows in spec-lang source only.
- **Guards must be pure and atomic with their use.** That holds inside one rule. A check in one rule and a use in another is a time-of-check gap unless a `law` connects them.
- **`@within` on managed and GPU targets is a measurement.**
- **The prompt-integrity rule is conservative.** Almost every useful model call mixes trusted instructions with untrusted text, so almost every output is untrusted and must be guarded. That is the intent, and it will feel heavy.
- **Uncheckable semantics stay uncheckable.** No guard decides whether a summary is faithful; that claim stays statistical.
