# Types and claims

> **Design.** Nothing here runs yet. This doc defines what a type is in spec-lang, which type features are core and which are library, and how each kind of claim is checked.

**Rule: a type is a set of values plus a set of claims. Every claim carries evidence.** How evidence is graded is [04-evidence](04-evidence.md). Effects, labels and trust are [05-effects-and-trust](05-effects-and-trust.md). Syntax is the [language card](01-language-card.md).

## Seven subjects, one mechanism

Every guarantee in the language is a claim. Claims differ only in what they are about.

| Subject | A claim about | Written as | Checked by |
|---|---|---|---|
| value | `self` | `T where p`, ranges, `@pre`, `@post`, `@guard` | structural check, then solver |
| grade | an index from an algebra | `Money[USD]`, `Vec3[m/s]`, `@pii`, `@within` | unification or lattice join |
| computation | what running does | `@uses`, `@fails`, `@alloc` | row inference |
| place | where a value lives | `own`, `ref`, `mut`, `layout` | borrow check |
| trace | sequences of states | `law f(t: Trace[a \| b])`, `flow` | induction, bounded search, simulation |
| run-pair | two runs compared | a `law` with two traces; `@idempotent`, `@tenant` | label typing, or search |
| distribution | many samples of a generator | a `law` over an eval set | sampling only |

The subjects are not interchangeable: one run cannot refute a run-pair claim, and a distribution claim is never true of one value. What is shared is the bookkeeping: one evidence scale, one ledger, one diagnostic shape.

## Core, library, excluded

| In the core | In the library or a vocabulary | Excluded |
|---|---|---|
| carriers: `Bool`, `Int`, `u8 ... i64`, `Float`, `Text`, `Bytes`, records, sums, functions, `List`, `Set`, `Map`, `Option`, `Id[T]` | SI units, `Money`, `Date`, `Instant` | type-level computation |
| `where` claims; `@pre`, `@post`, `@guard`; `law`; `flow` | `entity`, `layout`, `system`, `controller` | full dependent types |
| three grade algebras: group, lattice, ordered monoid; the labels `@pii`, `@secret`, `@tenant`; the budgets `@within`, `@alloc` | token and money budgets | inheritance |
| effect rows, handlers | consistency levels, CRDT laws | implicit conversion, null, exceptions |
| places: `own`, `ref`, `mut` | allocators, page tables, B-trees | user operators, overloading |
| evidence and the obligation pipeline | eval sets, scored outputs | |

Types depend on values only through claims and grade indices. A list of length `n` is `List[T] where self.size == n`, not a new type constructor. This keeps type inference decidable and puts all undecidability in claim discharge, where it is graded and reported.

Ownership is core for a reason that is not about systems code: a refinement on a mutable, aliased place is unsound, because another writer can break it. Above machine altitude values are immutable and state changes only inside an atomic `rule`, so the problem cannot arise. At machine altitude `own`, `ref` and `mut` restore the same guarantee.

## Value claims

```slg
type Age = 0..=150;                                       // a range is a type
type Cents = Int where self >= 0;                         // refinement
type Email = Text[3..=254] where self.matches(re"^[^@ ]+@[^@ ]+$");
type User(id: Id[User], email: Email, verified: Bool);
type Verified = User where self.verified;                 // a state type is a refinement
type Seat = Free | Held(by: Id[User], until: Time) | Sold(to: Id[User]);

@edge(Free -> Held, via: hold)
@edge(Held -> Sold, via: buy)
@edge(Held -> Free, after: 10min)
flow SeatLife(at: Seat);

@guard(users.get(by).is(Some(u)) and u.verified else NotVerified)
rule post(by: Id[User], text: Text[1..=500]) = { posts.push(Post(by, text)); }
```

| Feature | Rule |
|---|---|
| nominal | `Id[User]` and `Id[Task]` never unify. A record type is nominal; a `where` type is a subset of its base |
| subtyping | only by implication: `Verified` is usable as `User`; the reverse is an obligation |
| narrowing | a passed `@guard`, a `match` arm, `x.is(...)`, and `else fail` establish claims for the code after them |
| state machines | a sum type gives the states; a `flow` lists each legal `@edge`, with the rule, the caller and the delay that may take it; every writer of the place is checked against them, and an unlisted move is illegal |

**Obligations.** A claim becomes an obligation wherever a value must enter a narrower type: a call, a construction, an assignment to `state`. Each obligation ends in exactly one of three ways.

| Outcome | Meaning | Author sees |
|---|---|---|
| discharged | follows from guards, `@pre`, types in scope | nothing |
| refuted | a concrete input breaks it | an error with the shrunk counterexample |
| open | neither | `sketch`: the interpreter checks it on every run. `release`: an error naming the legal fixes |

The compiler never inserts a hidden runtime check. If `a.div(b, floor)` cannot show `b != 0`, the fixes offered are `@pre(b != 0)` (the caller's duty), `@guard(b != 0 else DivByZero)` (a typed refusal), or a proof. The choice changes the interface, so the author makes it.

## Grades

A grade is a type index drawn from a declared algebra. The checker is generic over three algebras; units, labels and budgets are instances.

| Algebra | Instances | Composition | Decidable |
|---|---|---|---|
| abelian group | dimensions, currencies | `×` adds exponents, `÷` subtracts | yes, with principal types |
| lattice | `@pii`, `@secret`, `@tenant(x)`, integrity, consistency | every operation joins its inputs | yes; value-indexed labels need equality reasoning |
| ordered monoid | `@within`, `@alloc`, stack, tokens, money spent | sequence adds, branches take the maximum, loops multiply by a bound | only when every loop has a bound |

```slg
const G: Vec3[m/s²] = (0, -9.81, 0)[m/s²];

fn fall(v: Vec3[m/s], dt: Time) -> Vec3[m/s] = v + G * dt;

fn convert(x: Money[EUR], rate: Float[USD/EUR]) -> Money[USD] = (x * rate).round(half_even);
```

A currency is a dimension with no conversion, so adding `Money[USD]` to `Money[EUR]` is a type error and a conversion needs a rate whose unit says what it is. `Int` is exact and unbounded; a backend picks a machine width from the value claims, and overflow is an obligation, never a behaviour.

## Money transfer

```slg
type Account(owner: Id[User], balance: Money[USD] where self >= 0USD);
state accounts: Map[Id[Account], Account];
const a1: Id[Account] = Id(1);
const a2: Id[Account] = Id(2);
const ana: Id[User] = Id(1);

@why("money moves; it is never created or destroyed")
@uses(accounts.write)
@access(user -> all)
@guard(accounts[from].owner == by else NotFound, to in accounts else NotFound, from != to else SameAccount)
@guard(n > 0USD else NotPositive, accounts[from].balance >= n else Insufficient)
@idempotent(key)
pub rule transfer(by: Id[User], key: Id[Transfer], from: Id[Account], to: Id[Account], n: Money[USD]) = {
  accounts[from].balance = accounts[from].balance - n;
  accounts[to].balance = accounts[to].balance + n;
}

law conserved(t: Trace[transfer]) =
  t.steps.all(s => s.after.values.sum(x => x.balance) == s.before.values.sum(x => x.balance));

@given(accounts[a1] -> Account(Id(1), 100USD), accounts[a2] -> Account(Id(2), 0USD))
test moves_once = {
  transfer(ana, key: Id(7), from: a1, to: a2, n: 30USD);
  transfer(ana, key: Id(7), from: a1, to: a2, n: 30USD);
  accounts[a1].balance -> 70USD;
  transfer(ana, key: Id(8), from: a1, to: a2, n: 500USD) -> fail Insufficient;
}
```

| Claim | How it is settled |
|---|---|
| no mixed currencies | grade unification; instant |
| `balance >= 0USD` after the write | follows from the last guard; linear arithmetic, expected `proven` |
| `conserved` | one rule, linear arithmetic, inductive; expected `proven` |
| no double spend on replay | `@idempotent(key)` derives a unique key; `proven`, trusting the store's uniqueness |
| isolation between concurrent transfers | "a rule is atomic" is a language rule; on a real store it rests on the backend, reported as such |
| a transfer that must call a bank | not a `rule`: a `task` whose steps each commit alone, so `conserved` must hold between steps too, usually through a `Pending` state |

Remove the last guard and the build does not say "might be negative". It returns the transfer that breaks the refinement.

## Places and layout

Above machine altitude ownership is never written. A `machine` body may use `own`, `ref` and `mut`, and a `layout` gives exact bytes. The crossing rule: a machine declaration exports an ordinary signature, and a place can be moved or lent for the length of a call but never stored in `state`, sent on the wire, or captured by higher code. A page allocator is called by a rule that does not know ownership exists.

```slg
use kernel;

@size(8B)
@endian(little)
layout Pte(present: Bit, write: Bit, user: Bit, _a: Bits[9], frame: Bits[40], _b: Bits[11], nx: Bit);

type Page = VirtAddr where self mod 4KiB == 0;
type Perm = PermBits where not (self.write and self.exec);
state spaces: Map[Asid, Space];

@uses(spaces.write, cpu.tlb, irq.off)
@fails(OutOfMemory)
@guard(not spaces[s].maps(va) else AlreadyMapped)
@post(
  spaces[s].walk(va) == Some((f, p)),
  old(spaces[s]).mapped.all(v => spaces[s].walk(v) == old(spaces[s]).walk(v)),
)
@within(5us)
@alloc(none)
pub rule map(s: Asid, va: Page, f: own Frame, p: Perm) = machine {
  let leaf = try spaces[s].descend(va);
  leaf.store(Pte(frame: f.number, present: 1, write: p.write, nx: not p.exec));
  flush_tlb(va);
}

law wx(t: Trace[map | unmap]) =
  t.states.all(w => w.values.all(s => s.mapped.all(v => not (s.pte(v).write and not s.pte(v).nx))));

law kernel_half_shared(t: Trace[map | unmap], a: Asid, b: Asid, va: KernelPage) =
  t.states.all(w => w[a].walk(va) == w[b].walk(va));
```

| Claim | Expected evidence | Why |
|---|---|---|
| `Pte` is 8 bytes with those bit positions | `proven` | computed by the compiler |
| a frame is mapped at most once | `proven` | `own Frame` is consumed by `map` |
| `wx` | `proven` | inductive from `Perm`; bit-vector arithmetic |
| second `@post` (no other mapping changes) | `bounded` | quantifies over a four-level tree in memory; needs an author-supplied abstraction to go higher |
| the processor honours the tables after `flush_tlb` | `assumed` | a hardware model, named in the trust set |

## Trace, run-pair and distribution claims

A `law` whose parameter is a `Trace` quantifies over every reachable interleaving of the named rules. Faults are events like any other: `Trace[put | commit | crash]`. A law with **two** traces is a run-pair claim, which is how isolation and determinism are stated; `@tenant` and `@idempotent` are the named shorthands. A law over an eval set is a distribution claim. Worked cases: crash consistency in [04-evidence](04-evidence.md), tenant isolation and model output in [05-effects-and-trust](05-effects-and-trust.md).

## Written and inferred

| The author writes | The compiler infers |
|---|---|
| types, guards, laws, tests | effect rows of private declarations |
| `@uses` and `@fails` on `pub` declarations | labels on every intermediate value |
| units on constants and signatures | units on every expression |
| a budget where one matters | cost of each callee, where a bound exists |

## Limits

- **The solver's fragment is small.** Linear arithmetic, equality, bit-vectors, finite collections and algebraic data are decidable. Nonlinear arithmetic, text predicates, unbounded quantifiers and heap shapes will often come back open. The language makes that visible; it does not make it go away.
- **Narrowing is local.** A claim established in one rule does not survive a write by another, or a step boundary in a `task`, unless it is a `law`.
- **The wrong law, proven, is still wrong.** Required witnesses for every `@pre` catch vacuous claims only.
- **Machine-altitude proofs are expensive.** The page-table frame condition above is the normal case, not the exception.
