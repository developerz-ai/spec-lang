# Type rules

> **Design.** Nothing here runs yet. This doc states the checker and the evaluator as judgments and rules, precisely enough that two teams would build the same one; where it is not yet precise enough, Limits says so.

The [language card](01-language-card.md) wins. Motivation: [03](03-types-and-claims.md), [04](04-evidence.md), [05](05-effects-and-trust.md), [27](27-authorization.md).

**Rule: phase 1 is structural, always terminates, and emits obligations. Phase 2 discharges them and may answer "open". Phase 1 never waits for phase 2.**

## 1. Notation

```text
Σ   module environment: visible declarations, kinds and notes
Γ   locals:  x : T @ m      m ∈ { val, var, own, ref, mut }
Φ   facts known here (a conjunction of Bool expressions)
K   enclosing kind ∈ { fn, rule, task, test, law }      A   altitude ∈ { define, machine }
ε   effect row: a finite set of labels  place.op | wait | alloc | float.native | ...
F   failure set: a finite set of error names
O   obligations: a set of  (Φ ⊨ φ, origin, code)

Σ ⊢ T type                         T is well formed
Σ ⊢ S <: T ⊣ O                     S may be used as T, given O
Σ;Γ;Φ ⊢ e ⇒ T ! ε;F ⊣ O            e synthesises T
Σ;Γ;Φ ⊢ e ⇐ T ! ε;F ⊣ O            e checks against T
Σ;Γ   ⊢ p ⇐ T ⊣ Γ';φ               pattern p matches T, binds Γ', and proves φ on success
Σ;Γ;Φ ⊢ s ⊣ Γ';Φ' ! ε;F;O          statement s
Σ ⊢ d ok ⊣ O                       declaration d
```

## 2. Names and `use`

```text
use a.b;              binds b to module a.b          (members as  b.name)
use a.b.c;            binds c to declaration c of a.b
use v;  (vocabulary)  adds v's kinds and notes

lookup of a bare NAME, first hit wins:
  1 locals and pattern bindings   2 parameters   3 this module   4 names bound by use   5 core library
a NAME found at two levels, or bound twice at one level           E-NAME-SHADOW / E-NAME-AMBIG
a declaration's kind is neither core nor imported                 E-KIND-UNKNOWN
the module graph of `use` has a cycle                             E-USE-CYCLE
a name of another module that is not pub                          E-NAME-PRIVATE
a write to state of another module (only its rules may)           E-STATE-FOREIGN
```

A vocabulary kind is checked on its expansion into core declarations; diagnostics map back to the source.

## 3. Types and subtyping

```text
T ::= Bool | Int | uN | iN | Float | Text | Bytes | Time | lo..=hi | lo..<hi
    | N | Id[N] | Option[T] | List[T] | Set[T] | Map[T, T] | (T, ..., T)
    | B[u]                       B numeric, u a unit                    (§5)
    | T where φ                  φ : Bool with `self` bound
    | (x: T, ...) -> T ! ε;F
    | own T | ref T | mut T      machine altitude only                  (§7)
    | Trace[r | ... | r]         test and law parameters only
    | Replicated[T, c]           state declarations only

base(T where φ) = base(T)        claims(T where φ) = claims(T) ∧ φ
a range lo..=hi  is  Int where lo <= self <= hi
```

```text
S-Refl      T <: T
S-Forget    (T where φ) <: T
S-Claim     S <: base(T)   ⊣ O
            ─────────────────────────────────────────────  x fresh
            S <: T  ⊣  O ∪ { Φ ∧ claims(S)[x] ⊨ claims(T)[x] }
S-Nominal   N <: N' only if N = N'.          Id[N] <: Id[N'] only if N = N'
S-Co        Option, List, Set, tuple: covariant.   Map: invariant in the key, covariant in the value
S-Fn        parameters contravariant, result covariant, ε and F by ⊆
S-Unit      B[u] <: B[v]  iff  u = v in the unit group
none        Int, uN, iN and Float never convert implicitly; conversions are library calls
```

Subtyping is structural; everything about claims leaves as an obligation.

## 4. Expressions and statements

Literals, lambdas, `[]`, `Map()`, `none` and holes are **checked**; everything else **synthesises**.

```text
T-Var     Γ(x) = T @ m ,  m ≠ moved                    ⊢ x ⇒ T (and T where self == x)
T-Lit     n : n..=n      10ms : Time      19.99USD : Money[USD]      9.81[u] : Float[u]
T-Call    Σ(f) = (x_i: T_i) -> R ! ε_f;F_f  with @pre P, @post Q
          e_i ⇐ T_i[e/x]      O += { Φ ⊨ P[e/x] }      Φ += Q[e/x, call/result]
          F_f joins the caller's F, unless `else`, try_call or a test arrow handles it here
          ⊢ f(e...) ⇒ R[e/x] ! ε_f ∪ ε_args ; F as handled
T-Index   m ⇒ Map[K, V] ,  k ⇐ K
          Φ ⊨ k in m is an obligation of code INFO only: if open, F += Missing     ⊢ m[k] ⇒ V
T-Else    e ⇒ T ! F ,  d ⇐ T (or d = fail E)          ⊢ e else d ⇒ T ! F minus {Missing, none} (plus E)
T-Fail    E names an error; a new name declares itself ⊢ fail E ⇐ any T ! {E}
T-If      c ⇐ Bool ;  a under Φ ∧ c ;  b under Φ ∧ ¬c ;  result is the join of both branches
T-Is      e ⇒ T ,  p ⇐ T ⊣ Γ';φ                       ⊢ e.is(p) ⇒ Bool ; where it holds, Γ' and φ are in scope
T-Match   every arm p_i ⇐ T ; arms exhaustive and none dead (§6) ; result is the join
T-Lam     checked against (x: S) -> R ;  `it` is the one parameter of an unnested lambda
```

A join needs equal bases and keeps only claims both sides carry. Generic parameters are instantiated by first-order unification of argument types, left to right. A parameter that appears only in the result must be written (`parse[Select](raw)`), else `E-TYPE-AMBIG`. With no overloading and no implicit conversion, synthesis is unique.

```text
St-Let      e ⇒ T                         Γ' = Γ, x : T @ val          x already in Γ: E-NAME-SHADOW
St-Var      as St-Let with @ var ;  a var is not captured by a lambda  (E-PLACE-CAPTURE)
St-Assign   place ⇒ T ,  e ⇐ T ,  place is a var, or state inside a rule, or reached through mut
St-IfFail   if c { ... fail E }  with no else                          Φ' = Φ ∧ ¬c
St-Assert   assert e :  a check in a test; elsewhere O += { Φ ⊨ e } and Φ' = Φ ∧ e
St-Return   e ⇐ declared result ;  O += { Φ ⊨ @post[e/result] }
```

A write to `state` kills the facts in Φ that mention that path.

## 5. Claims and units

Obligations arise at exactly these points:

```text
O-Sub     a value enters T where φ: argument, return, construction, assignment to state      E-CLAIM
O-Pre     a call:  callee's @pre with arguments substituted                                   E-CLAIM-PRE
O-Post    a return:  @post with result and old(...)                                           E-CLAIM-POST
O-Law     per rule r and law L whose Trace names r:  L(before) ∧ step_r ⊨ L(after)            E-LAW
O-Flow    per write to a flow place by rule r:  (old, new) is an edge with via: r             E-FLOW-MOVE
O-Budget  per @within / @alloc:  cost(body) ≤ limit                                           E-BUDGET
```

Facts enter Φ from: parameter claims, `@pre`, a passed `@guard` or `@access`, the true side of a condition, `St-IfFail`, `else fail`, a `match` arm, a callee's `@post`. Phase 2 answers `proven`, `refuted` (always an error, with a counterexample) or `open` (an error only under `release`, [04](04-evidence.md)).

Units form the free abelian group over base symbols; a unit is a map from symbol to integer exponent.

```text
U-Mul    B[u] * B[v] : B[u+v]         U-Div   B[u] / B[v] : B[u-v]        U-Pow   x² : B[2u]
U-Add    B[u] + B[u] : B[u]           unequal units: E-UNIT-MISMATCH
U-Zero   B[0] is plain B ;  Time / Time : Float ;  Int * Time : Time ;  70% : scale 1/100, unit 0
U-Var    unit variables in generics are solved by abelian-group unification (decidable, principal)
U-Money  Money[c] is Decimal at the currency's scale; * and / give an unrounded Decimal[c];
         storing it as Money[c] needs .round(mode)                        E-UNIT-ROUND
```

## 6. Patterns

```text
p ::= _ | x | literal | C(p, ...) | (p, ...)

P-Bind    x ∉ dom(Γ)                 ⊢ x ⇐ T ⊣ x : T ; true
P-Same    x ∈ dom(Γ) , Γ(x) ~ T      ⊢ x ⇐ T ⊣ · ; self == x          never both: nothing shadows
P-Ctor    C a variant of T with fields T_i ,  p_i ⇐ T_i               ⊢ C(p...) ⇐ T ⊣ ∪Γ_i ; self is C ∧ φ_i
```

Exhaustiveness is decided over variants, `Bool`, `Option`, tuples and integer intervals; a `P-Same` or literal arm never completes a type alone. Missing case: `E-MATCH-MISSING` with a witness. Dead arm: `E-MATCH-DEAD`.

## 7. Places and `mut`

```text
above machine   every local is val or var. A parameter marked mut names a place the callee may write.
  M-Arg         the argument is a place expression: a var, or a state path inside a rule
  M-Disjoint    two mut arguments of one call have different roots or provably different indices
                (syntactic prefix test; unequal indices become an obligation)          E-PLACE-ALIAS
machine         own T is affine; ref T shared; mut T unique
  M-Move        using x : own T as a value sets x to moved; a later use: E-PLACE-MOVED
  M-Borrow      ref e / mut e are legal only as call arguments and last for that call.
                No reference is stored, returned or captured, so no lifetime is inferred or written.
  M-Cross       own/ref/mut types may not appear in state above machine, events,
                or a pub signature except as own parameter or result             E-PLACE-ESCAPE
```

## 8. Effects and failures

```text
E-Sub     an expression's row is the union of its parts' rows
E-Kind    fn    ε ⊆ { s.read }                     "pure" for @memo and the build means ε = ∅
          rule  wait ∉ ε ; an outside effect in a rule is a send after commit and has type Unit
          task  any ε
          law:  ε ⊆ { s.read }.   @access, @guard, @pre, @post:  ε ⊆ { s.read, clock.read }   E-EFFECT-KIND
E-Call    a rule calls fn and rule; a task calls fn, rule, task; t.start(args) is legal in both
```

**Inference.** For private declarations ε and F are the least fixed point over the module's call graph (finite sets, monotone union, so it terminates). For `pub` declarations both are written and `inferred ⊆ written` is checked (`E-EFFECT-UNDECLARED`, `E-FAIL-UNDECLARED`). The failure set of a declaration is: its access error, its guard errors, `@fails`, and `Missing` if an index obligation is open. Labels (`@pii`, `@secret`, `@tenant`, field visibility) are inferred per expression as the join of its operands and enclosing conditions, and checked at each exit (`E-LABEL-FLOW`).

## 9. Notes, holes and `pub`

```text
note declaration    @allows(k_1, ..., k_n)  note n(x_i: T_i) = meaning
N-Use               n ∈ Σ  (E-NOTE-UNKNOWN) ;  kind(d) ∈ allows(n)  (E-NOTE-PLACE)
                    arguments ⇐ T_i in the scope of d: its parameters, plus
                    `self` on a type, `result` and old(...) in @post, `by` in @access
N-Once              a note appears once per declaration unless declared repeatable
                    (@guard, @edge, @access clauses, @retry)                           E-NOTE-DUP
N-Clash             pairs declared contradictory, e.g. @memo with @alloc(none)        E-NOTE-CLASH
N-Access            a declaration someone can call in to (a calling trigger, an app surface) has @access;
                    one with no `by` and no way in has none                          E-ACCESS-MISSING / -ORPHAN
```

```text
H-Hole     ?h ⇐ T  for any T, with ε = F = ∅ and no obligations; in synthesis position
           without an expected type: E-HOLE-TYPE (annotate the let or the result)
           claims that depend on ?h are `stated`.     sketch: allowed.     release: E-HOLE

Pub        a pub declaration writes every parameter type, its result, @uses and @fails.
           Its interface: kind, name, generics, parameters with claims, result, the notes
           @pre @post @uses @fails @access, labels, and the laws naming it.
           Callers are checked against the interface only; a body change keeps its hash.
```

## 10. Evaluation

```text
World  W = (σ, now, Q, J, out, tr)
  σ    state of every module          Q    pending deliveries, task runs and due deadlines
  J    task journals                  out  sends awaiting dispatch          tr   the trace so far
```

**Rules.** A call `r(v...)` is one step of the world.

```text
R-Order    decode parameters ; @access (a Missing inside it is denial) ; flow edge `by` ;
           @guard clauses left to right ; body.     The first failure wins.
R-Commit   the body ends with value v, local store σ', emitted events E, starts S, sends O:
           W → (σ', now, Q ++ deliveries(E) ++ S, J, out ++ O, tr ++ [step])   reply v
R-Fail     any failure e, including IllegalMove:    W → W with tr ++ [step], before = after, reply fail e
R-Join     a rule called inside a rule runs in the caller's local store; its failure is the caller's
step       (event = r(v...), reply, at = now, before = σ, after = σ')
```

Laws never change behaviour: a law false after a commit is a refutation reported against that step, and a monitor in generated software.

**Events and time.**

```text
Ev-Emit      e.emit(v) inside a rule appends to E; nothing is visible before R-Commit
Ev-Deliver   an item (listener, v) of Q runs the listener as its own R-step, then leaves Q
Ev-Again     a delivered item may be delivered once more (at least once)
Ev-Order     items of one commit reach one listener in emission order; nothing else is promised
Tm-Advance   clock.advance(d): now += d; every `every`, `tick`, flow deadline and retry backoff falling due enters Q in time order
```

**Tasks.** A run is `(key, task, args, attempt, journal)`. A *step* is a rule call, an outside call, a clock or random read, or an `all`/`race` block; each has index `i` in program order.

```text
T-Start     enqueued at the starter's commit; key = @idempotent key, else fresh
T-Step      evaluate to the next step i; perform it with step key (key, i); append (i, result) to the journal
T-Crash     the continuation is lost; J is kept
T-Resume    re-evaluate the body from the start; a step with a journal entry returns that entry
            and performs nothing. A step without one is performed again with the same step key
T-Fail      failure e ends the attempt. If @retry matches e and attempt < n: after backoff, a new
            attempt with an empty journal and the same key. Otherwise the run ends with e and
            `exhausted`, if named, is enqueued
T-Done      a success is stored under key: a later start with the same key and payload replays it,
            a different payload fails KeyReused
T-All       branches interleave; when one fails the rest are cancelled at their next step boundary;
            the result is the lowest-source-index failure
T-Race      the first success wins, the rest are cancelled; if all fail, the lowest-index failure
```

Between steps a task body must be deterministic, which is why clock and random reads are steps. A step unsafe to repeat under `T-Fail` is `E-OPEN`.

**Tests.**

```text
Ts-Given    build W0: @given items left to right; a test name contributes its final world; handlers install
Ts-When     call;              perform the step, then run Q to quiescence in seeded order (time does not move)
Ts-Then     call -> v;         as Ts-When, then reply == v.        e -> v;  on a non-call compares values
Ts-Refuse   call -> fail E;    the reply is failure E (and its data, if written)
Ts-Blocked  reaching ?h ends the test as Blocked
Ts-Prop     parameters are generated from types and claims; failures are shrunk and stored
```

**Traces.** `Trace[r_1 | ... | r_n]` is the set of finite `tr` produced from a start world by choosing any enabled step each time: a call of some `r_i` with generated arguments, `Ev-Deliver`, `Ev-Again`, or a handler event such as `crash`. The start world is the test's `@given`, or for a `law` any world satisfying the types, flows and laws (so `O-Law` is an induction step). `t.eventually` and `t.within` use fair traces only.

## 11. What is decidable

| Question | Status |
|---|---|
| names, kinds, notes, subtyping, synthesis, patterns | decidable |
| units | decidable; unique most general solution |
| effect and failure rows, labels | decidable; least fixed point over finite sets |
| places: moves, borrows, aliasing by path | decidable; unequal-index aliasing becomes an obligation |
| obligations in linear arithmetic, equality, bit-vectors, finite sums and records, bounded quantifiers | decidable; may exhaust the step budget, which reports `open` |
| obligations with nonlinear arithmetic, text, unbounded quantifiers, recursion | undecidable; `open` is expected |
| `O-Law` | the induction step is an obligation as above; reachability-exact checking is bounded only |
| liveness, cost on real hardware, side channels | not decided here |
| termination of `fn` | syntactic: structural recursion or bounded loops; otherwise the `fn` may not appear in a claim |

## 12. Error codes

| Family | Codes |
|---|---|
| names | `E-NAME-UNKNOWN` `E-NAME-SHADOW` `E-NAME-AMBIG` `E-NAME-PRIVATE` `E-USE-CYCLE` `E-KIND-UNKNOWN` `E-STATE-FOREIGN` |
| notes | `E-NOTE-UNKNOWN` `E-NOTE-PLACE` `E-NOTE-DUP` `E-NOTE-CLASH` `E-NOTE-ARG` |
| types | `E-TYPE-MISMATCH` `E-TYPE-AMBIG` `E-TYPE-ARITY` `E-MATCH-MISSING` `E-MATCH-DEAD` |
| units | `E-UNIT-MISMATCH` `E-UNIT-ROUND` |
| places | `E-PLACE-MOVED` `E-PLACE-ALIAS` `E-PLACE-ESCAPE` `E-PLACE-CAPTURE` `E-PLACE-ASSIGN` |
| effects | `E-EFFECT-KIND` `E-EFFECT-UNDECLARED` `E-FAIL-UNDECLARED` `E-LABEL-FLOW` |
| access, flows | `E-ACCESS-MISSING` `E-ACCESS-ORPHAN` `E-FLOW-MOVE` `E-FLOW-EDGE` |
| claims | `E-CLAIM` `E-CLAIM-PRE` `E-CLAIM-POST` `E-LAW` `E-BUDGET` (each: `refuted`, or `open` under `release`) |
| holes, choices | `E-HOLE` `E-HOLE-TYPE` `E-OPEN` |

These refine the list in [07](07-agent-loop.md), which stays the one a model is shown: a code here is reported under its family there (`E-NAME-*` and `E-KIND-UNKNOWN` as `E-NAME`, `E-NOTE-*` as `E-NOTE`, `E-ACCESS-*` as `E-ACCESS`, `E-STATE-FOREIGN` as `E-OWNER`, `E-USE-CYCLE` as `E-CYCLE`, the types, units, places, effects, flows and claims families as `E-TYPE`), with the finer code as a field. Every diagnostic carries the code, the span, the failed judgment, a counterexample where one exists, and the fixes [07](07-agent-loop.md) allows.

## Limits

- **Not yet two-team precise:** label joins across `match` arms, cost composition for `O-Budget`, and the expansion contract of vocabulary kinds are only outlined.
- **Failures pass up without a mark.** A call site does not show that it can fail; only the signature does. That is shorter to write and easier to miss.
- **Second-class borrows** (`M-Borrow`) remove lifetime inference at the price of expressiveness: an iterator holding a reference cannot be written without `own` or an index.
- **Fact killing on state writes** is by syntactic path: sound and coarse. Joins drop claims.
- **`Ts-When` runs to quiescence**, hiding interleavings between a call and its deliveries; only traces explore those.
- **`T-Resume` assumes determinism between steps.** A `foreign` call in a task that is not a step breaks replay undetected.
