# Vocabularies

> **Design.** Nothing here runs yet. What a vocabulary package is, how it stays total and hygienic, and how two vocabularies share one declaration.

**Rule: a vocabulary adds kinds, notes, handlers and an app shape, nothing else.** No grammar, no parser hook, no build script, no runtime of its own. Every kind is shorthand for ordinary declarations; every note is one typed policy slot. `spec-lang view --as expanded` shows the shorthand written out.

## The parts

A vocabulary is a module of ordinary declarations: the `vocab.slg` of a library `libs/<name>/`, beside its `contract.slg` and `pretend.slg` ([card, Libraries](01-language-card.md#libraries); [008](../decisions/008-libraries-ship-their-spec.md)). Each part uses a form from the [language card](01-language-card.md).

| Part | Written as | The compiler uses it for |
|---|---|---|
| Kinds | `kind` | new declaration words: `entity`, `query`, `system`, `layout` |
| Notes | `note` | new policy slots: `@page`, `@index`, `@replicas`; with `@allows(on)`, a trigger: `@on(cli("add"))` |
| Typing | parameter types on the kind or note, plus `@allows` | rejecting a misplaced or ill-typed use before expansion |
| Expansion | a pure `fn` from the declaration to lower declarations | the meaning; what the interpreter runs |
| Laws | `law` | proof facts, rewrite rules and parity tests, from one statement |
| Effects | `handler out { rule line(text: Text); test text -> Text; }`, and in `pretend.slg` a `handler` whose arms implement each operation | the model the interpreter and tests run against; an operation's kind says who may use it (`rule`: rules, `fn`: any body, `test`: tests) |
| App shape | `app Game(start: rule, @on(tick) tick: rule, ...)` | the parameters an `app` takes in a module that uses the vocabulary, and the trigger a named rule must carry |
| Suite | `test` | what every backend claiming the kind must pass ([11](11-backends-and-parity.md)) |
| Explanation | `@explain` | the human projection of each use ([10](10-compiler-and-ir.md)) |

A vocabulary never ships target code. Implementations belong to backends, so a vocabulary cannot smuggle a runtime in.

The notes that describe a vocabulary are not core either. Apart from `@allows`, they are declared by the `vocab` vocabulary that every vocabulary `use`s:

```slg
@allows(kind, note) note reads(aspects: List[Aspect]) = footprint;
@allows(kind, note) note writes(aspects: List[Aspect]) = footprint;
@allows(kind) note explain(text: Text) = explanation;
@allows(note) note default(policy: Policy) = default_policy;
@allows(kind) note bridges(a: Kind, b: Kind) = bridge;
```

Then a slice of `data`:

```slg
use vocab;

@allows(module)
@reads(fields)
@writes(storage, identity, access)
@explain("{name}: a stored record. {access}")
kind entity(fields: List[Field]) -> List[Decl] = expand_entity;

fn expand_entity(e: Entity) -> List[Decl] = [
  record_type(e),
  store_state(e),
  crud_rules(e),
  access_law(e),
];

@allows(rule, query)
@default(keyset)
note page(size: 1..=500, by: PageBy = keyset) = page_policy;

law pages_cover(q: Query, n: 1..=500) =
  q.stable implies q.pages(n).flat_map(it.rows) == q.rows;

law filters_fuse(q: Query, p: Row -> Bool, r: Row -> Bool) =
  q.filter(p).filter(r) == q.filter(x => p(x) and r(x));

@given(notes.seed(120))
test keyset_survives_insert = {
  let first = list_notes(page: 1);
  notes.add(text: "new");
  assert list_notes(after: first.cursor).rows.all(not (it in first.rows));
}
```

## One law, three uses

| Use | Mechanism |
|---|---|
| Proof fact | the prover may assume a vocabulary law once the law itself has evidence |
| Rewrite rule | an equation is an optimiser rule at the IR layer where its kinds are still visible |
| Parity test | the law is sampled against every backend that claims the kind |

The optimiser cannot use a rule nobody checked, because there is no second place to write one. A law's evidence follows [04-evidence](04-evidence.md); a rewrite built on a `sampled` law makes every claim it touches trust that law.

## Kept total, hygienic and typed

| Property | Mechanism |
|---|---|
| Terminating | an expansion `fn` is pure, uses structural recursion over its own declaration and bounded folds; no `while`, no effects |
| Stratified | an expansion may emit only core kinds and kinds from vocabularies it `use`s; the `use` graph is acyclic, so depth is bounded |
| Hygienic | an expansion sees its own declaration and the typed interfaces of declarations it names; never bodies, never names in scope |
| Ownership | expanded declarations land in the module that wrote the kind, so that module owns any `state` they create and only its rules write it; an expansion cannot add a writer to another module's state |
| Stable names | generated names are derived from the declaration's identity and a role (`Note.store`, `Note.create`); no counters |
| Typed once | expansion output is checked against the lower vocabulary for every shape the kind's parameters allow, when the package is built |
| No ambient power | opening, viewing and checking a module run no package code outside this machinery ([08](08-approval-and-anti-cheating.md)) |

"Typed once" is undecidable in general. Where the generic check fails, the package is marked `checked-per-use` and its errors must name the vocabulary, not the user's line.

## Notes are closed and typed

| Rule | Mechanism |
|---|---|
| One meaning | a note name resolves to exactly one `note` declaration among the `use`d vocabularies; two candidates is `E-NOTE-AMBIGUOUS` |
| Placement | `@allows(kinds)` lists where it may appear; elsewhere is an error |
| Arguments | typed like any parameter; ranges and enums bound them |
| Contradiction | two notes whose policies exclude each other fail with both sites named |
| Default | `@default(x)` names the policy used when the note is absent; `spec-lang view --as policy` lists every default in force |
| Pin | a named argument selects a strategy (`@page(50, by: offset)`); see the planner in [10](10-compiler-and-ir.md) |

Plumbing is a default policy, never source. A changed default in a package upgrade is a guarantee diff, not a silent change.

## Two vocabularies on one declaration

Each kind and note declares a **footprint** over a small fixed set of aspects owned by the core `data` vocabulary: `fields identity storage index lifecycle changes access placement`.

```text
data.entity      writes {storage, identity, access}  reads {fields}
sync.live        writes {changes}                    reads {fields, storage}
search.indexed   writes {index}                      reads {fields}
audit.logged     writes {changes}                    reads {fields, access}
```

| Case | Result |
|---|---|
| Disjoint writes | compose with no ceremony. The compiler expands in both orders and compares the hashes of the result; a difference is a package bug reported against both packages |
| One writes what another reads | ordered: writer first. A cycle is an error |
| Overlapping writes | need a **bridge**: a vocabulary that `use`s both and says how they share the aspect, with its own laws and tests |

No bridge, no build:

```json
{"code": "E-VOCAB-OVERLAP",
 "aspect": "Note.changes",
 "writers": ["sync.live", "audit.logged"],
 "fix": [{"use": "sync_audit", "why": "bridge: one change stream, audit reads it"},
         {"remove": "@logged", "at": "notes.slg:4"}]}
```

A bridge is ordinary vocabulary source:

```slg
use sync;
use audit;

@bridges(sync.live, audit.logged)
@writes(changes)
kind live_logged(of: Entity) -> List[Decl] = one_stream;

law audit_sees_all(t: Trace[change]) =
  t.steps.all(s => s.after.audit.contains(s.event));
```

## Versions

| Rule | Mechanism |
|---|---|
| Identity | a package is its content hash; `spec-lang.lock` pins it |
| Change class is computed | removed or weakened law, narrowed note range, changed default: breaking. New kind or note: additive. Authors do not choose the number |
| Upgrade | `spec-lang build --upgrade data` re-expands, re-runs the suite and prints the guarantee diff; nothing changes until it is landed |
| Coexistence | two versions of one vocabulary cannot annotate the same declaration; across modules they meet only through boundary types |

## Ship order

Order follows the judged ranking in [16-corpus](16-corpus.md): removable share, times code carrying the shape, times recurrence. The ranking orders work; it is not a measurement of spec-lang.

| Order | Vocabulary content | Kinds and notes | Note |
|---|---|---|---|
| 1 | entity, resource and one contract projected to storage, wire, clients, docs | `entity` `query`, `@access` `@tenant` `@page` `@index` | first backend is Ultimate ([12](12-frameworks.md)) |
| 2 | tests beside logic with effect handlers | none: core `test` and `handler` | ships with the interpreter, before any vocabulary |
| 3 | lifecycle | core `flow`, `@edge` | core; nothing to ship beyond the interpreter |
| 4 | effect policies | `@retry` `@idempotent` core; `@outbox` `@single_flight` `@deadline` | must state what is assumed of the outside world |
| 5 | typed failure with code, cause, fix, status | an `error` kind with `@status` `@fix` | cheap; wording stays hand-written |
| 6 | job, retention, schedule | core `task`, `event`, `@on(every(...))`; `@retain` | good first demo; needs a backend with a journal ([11](11-backends-and-parity.md)) |
| 7 | adapters to third parties | `adapter`, `@auth` `@limit` `@maps` | hardest to make honest; needs 4 |
| 8 | one source, many targets | boundary types | pays only with a second backend |
| 9 | workload shape | `service` `network`, `@replicas` `@scale` | independent and small |

Every note named in this table is a `note` declaration in its vocabulary, typed and placed like `page` above ([29](29-first-vocabularies.md)); none is core unless marked so.

Not early: a general UI vocabulary. Presentation code is large and was not shown to be mechanical. `game`, `kernel`, `storage`, `control` and `lang` follow the roadmap in [20](20-roadmap.md).

## Limits

- **Only part of this runs.** The toolchain loads `libs/<name>/` and checks notes on an `app`, triggers, handlers with their pretends, and an app shape. Kinds, expansion, laws and suites in a vocabulary, footprints, bridges and versions are design.
- **Footprints are coarse.** Eight aspects will produce false overlaps, and two vocabularies can still interfere through shared runtime state that no aspect names. The both-orders check catches only expansion-time interference.
- **Bridges can multiply.** Worst case is one per pair of writers on an aspect. The mitigation is that most vocabularies only read; whether that holds is unknown until several exist.
- **A wrong expansion is a wrong meaning** that every test agrees with. The suite is written by the same authors as the expansion; independent review of vocabularies matters more than of application code.
- **Generic checking of expansions is undecidable.** `checked-per-use` packages will leak vocabulary-internal errors to users.
- **Writing a vocabulary is compiler work.** Laws, a suite, a simulated handler and explanations are the price of every new word. The set of good vocabularies will be small for a long time.
