# The agent loop

> **Design.** Nothing here runs yet. How an agent reads, edits, asks and merges: six verbs, one response format, sufficient context slices.

**Rule.** Agent wall time is mostly model turns, so the unit to optimise is information per turn, not milliseconds per check.

## Six verbs

| Verb | Does |
|---|---|
| `view` | read: slice, hole, why, impact, policy, projection. Never executes project code |
| `edit` | change source; returns type errors, affected tests, counterexamples and questions in the same response |
| `test` | run, replay, explore seeds, mutate |
| `prove` | climb the evidence ladder under a deterministic step budget |
| `build` | generate the software and migrations |
| `land` | guarantee diff, authority check, attestation ([08](08-approval-and-anti-cheating.md)) |

The same six are the MCP tools; a vocabulary adds selectors, never a tool. Output is JSON; a large payload becomes a handle, never a truncation.

Editing files with ordinary tools stays legal; a daemon parses the change into the same operations.

## An edit is a question with several answers

```json
{"verb":"edit","target":"transfer","base":"b:41f0",
 "reads":{"accounts":"i:22","conserved":"l:09"},
 "alternatives":["@guard(from != to else SameAccount)","if from == to { return; }"]}
```

Each alternative runs in an overlay. The reply gives every pass vector and **the input on which they differ**. The disagreement is often the requirement nobody stated. Nothing is written until the author picks.

## Diagnostics

| Field | Contract |
|---|---|
| `code` | stable; one table below |
| `counterexample` | a minimal replayable trace, not a formula; `start` says which calls reach its first world, or that none was found ([06](06-logic-and-tests.md)) |
| `suspects` | a **set**: body, law, example, assumption. A contradiction does not say which side is wrong |
| `fixes` | edits already type-checked and run; each labelled `strengthens`, `preserves`, `weakens` or `unknown` |
| `evidence` | the level before and after, with its trust set ([04](04-evidence.md)) |
| `cost` | solver steps, tests re-run, context tokens |
| `lesson` | the card line and one minimal correct form |
| `related` | collapsed consequences of the same root |

A full one is in [06](06-logic-and-tests.md). When no edit preserves the promise, `otherwise` names the exit: `land --dispute sold_is_final`. Fixes marked `weakens` or `unknown` are never applied automatically. No fix is offered that was not run. Package text appears by handle. **A terse error can be worse than none**: in an ablation with obfuscated names, minimal type errors gave Claude Haiku 4.5 54.0%, no types 62.8%, full detail 69.7% ([table 5](https://arxiv.org/html/2606.01522v1)).

### Codes

The one list. A vocabulary may add codes under its own name.

| Code | Raised when | The fix it carries |
|---|---|---|
| `E-PARSE` | not in the grammar | nearest legal form |
| `E-NAME` | unknown name | nearest by spelling and meaning |
| `E-TYPE` | claim, unit, label or effect does not fit | the edit at the narrower side |
| `E-NOTE` | unknown, misplaced or contradictory note | the note that means it |
| `E-NOTE-AMBIGUOUS` | two vocabularies define the note | qualified name |
| `E-ACCESS` | a caller can reach it and no `@access` says who | a list of subject `fn`s in scope; never a default |
| `E-OWNER` | writes a `state` another module owns | the owner's `pub` rule, or the shape of one |
| `E-CYCLE` | `use` forms a cycle | the edge an `event` could replace |
| `E-OPEN` | two behaviours remain | at most three, each with its consequence |
| `E-CASE` | a case, scenario or property fails | input, expected, actual, edge |
| `E-LAW` | a law is refuted | trace, start, edit or missing law |
| `E-HOLE` | a hole blocks a build | type, needs, live values |
| `E-VACUOUS` | a claim holds of nothing | the premise nothing satisfies |
| `E-SPECIAL` | a branch matches only a test's literal | perturbed inputs |
| `E-WORLD` | a `@given` world breaks a law or flow | the law, and the nearest legal world |
| `E-STALE` | something read has changed | rebased edit, if it rechecks |
| `E-WEAKER` | `land`: weaker, newly assumed or unanswered, without its owner | who may sign ([08](08-approval-and-anti-cheating.md)) |
| `E-DATA` | stored values do not fit | repair hole and plan ([22](22-lifecycle.md)) |
| `E-VOCAB-OVERLAP` | two vocabularies govern one thing | the bridge to declare ([09](09-vocabularies.md)) |
| `E-NO-STRATEGY` | nothing implements a declared body or policy | which claim excluded each candidate ([10](10-compiler-and-ir.md)) |
| `E-PLAN-REFUSED` | no target keeps a guarantee | the claim, and targets that would ([11](11-backends-and-parity.md)) |
| `BACKEND.BUG` | target toolchain or parity fails | none: a backend defect, never author work |

Errors a *program* raises (`Missing`, `NotFound`, `IllegalMove`, `KeyReused`, its own) are values, not codes. They appear inside `E-CASE`.

## Errors a small model can act on

The contract, for every error:

| # | Rule |
|---|---|
| 1 | one root cause; consequences are counted, not printed |
| 2 | the failing input, what happened, what was expected |
| 3 | the exact replacement text, already parsed, typed and run; if several are valid, at most three, each with its consequence |
| 4 | the one card line that applies, inline |
| 5 | never only "type mismatch"; an unknown name gets the nearest legal ones, by spelling **and** by meaning |
| 6 | if the choice is about intent, it names who may answer |

**Unknown note.**

```slg
@require(amount > 0 else Empty)
rule withdraw(id: Id[Account], amount: Int) = { accounts[id] = accounts[id] - amount; }
```

```text
E-NOTE  line 1  `@require` is not a note.
input    withdraw(#1, amount: 0)
now      does not compile; without the note it succeeds and changes nothing
fix A    @guard(amount > 0 else Empty)   callers get `fail Empty` at run time
fix B    @pre(amount > 0)                callers must prove it; a call that cannot is an error there
card     @guard(p else Err): runtime refusal with a typed error, checked before the body
checked  A and B: types, 3 tests.  B makes 1 caller an error: batch.slg line 12
```

`else Empty` fits only A, so A is first. `@require` maps by meaning, from a synonym table.

**Guard boundary.** The tests are those of [06](06-logic-and-tests.md); the body has `<=`.

```slg
@guard(invites[t].is(Pending(until)) and now() <= until else Gone)
rule accept(by: Id[User], t: Token) = {
  invites[t] = Accepted(by);
  members.insert(by);
}
```

```text
E-CASE  closed_from_day_seven  accept(#1, t: t1)
input    invite t1 is Pending(until: 7d); now is 7d  (derived: the guard's edge; 8d to 30d pass)
now      ok; invites[t1] is Accepted(#1)
expected fail Gone
edge     last accepted 7d, first refused 7d + 1ms.  The test wants first refused 7d
fix      now() < until            on accept, guard 1      checked: types, 4 tests green
not      `late: 7d..=30d` -> `late: 8d..=30d`: would weaken a test; needs its owner
card     Guards run first, top to bottom. The first that fails gives the error
```

`not` is the tempting wrong edit, never listed under `fix`.

**Open choice.**

```slg
@cases((6, 3) -> 2)
fn share(total: Int, people: Int where self > 0) -> Int = total / people;
```

```text
E-OPEN  line 2  `/` on Int has more than one result
input    share(7, people: 2)      your case (6, 3) -> 2 fits all of these
fix A    total.div(people, floor)      3     the remainder 1 is dropped
fix B    total.div(people, ceil)       4     shares add up to more than total
fix C    total.div(people, half_even)  4     may be more or less than total
card     on Int use a.div(b, floor), because / on Int is an open choice
who      the owner of `share`, or a person, by adding a case such as (7, 2) -> 3
```

A case is the better answer: it outlives the body. The compiler knows a **closed list** of choice points: `/` on `Int`, derived bodies with several solutions, ties in an observed order, same-instant rules whose order shows, a retried step that is not safe to repeat, overlapping policies, a missing default, a lossy migration. Choices outside the list are found only by search ([08](08-approval-and-anti-cheating.md)).

### The same error again

The daemon fingerprints (session, declaration, root cause, counterexample shape).

| Time | Help |
|---|---|
| 1 | the contract above |
| 2 | what the last edit changed, why the same input still fails, and the **whole corrected declaration** instead of a fragment |
| 3 | the run of that input step by step with values, and the smallest wrong expression replaced by a hole: the question becomes one expression of a known type |
| 4 | stop. Report the replay and the attempts to a stronger model or the law's owner, with `land --dispute` filled in |

A different counterexample is progress and resets the count. No step relaxes a law.

## Finding your way

A program is a tree of modules ([26](26-program-structure.md)); path is name. No directory walk or grep is needed. (Token figures illustrative.)

```json
{"view":"map","tokens":96,
 "apps":[{"name":"Shop","target":"ultimate","exposes":["orders.place","orders.cancel","items.reserve"]}],
 "modules":[
  {"path":"sales.orders","why":"a cart becomes a promise","state":["orders"],"pub":["place","cancel","ship"],
   "uses":["stock.items"],"emits":["placed"],"hears":[],"laws":3,"holes":0,"maxEditCost":1480},
  {"path":"stock.items","state":["free"],"pub":["reserve"],"uses":[],"emits":["reserved"],
   "hears":["sales.orders.placed"],"laws":1,"holes":1,"maxEditCost":620}]}
```

| Step | Ask | Answers |
|---|---|---|
| 1 | `view map` | which module owns this state, says this reason, hears this event |
| 2 | `view sales.orders --as contract` | `pub` signatures, errors, effects, laws, events; no bodies |
| 3 | `view slice place` | what one edit needs |
| 4 | `view impact place` | who calls it, hears it, exposes it, tests it, and which laws could move |

`view where "text"` searches names, `@why`, errors and laws, and returns declarations, not lines. A module owns its state, so "who can change this?" is answered by one contract.

**Adding a module**, contract first:

| Step | Content | The reply checks |
|---|---|---|
| 1 | types, `state`, `pub` signatures with `?holes`, laws, two or three tests | `E-CYCLE`, `E-OWNER`, name clashes; tests report `Blocked` |
| 2 | neighbours: `use`, or an `event` where `use` would cycle | who hears it; that it runs later, at least once |
| 3 | fill holes, one per edit, possibly by a smaller model | [06](06-logic-and-tests.md) |
| 4 | expose: one line in an `app`, with `@access` on each exposed rule | `E-ACCESS`; the guarantee diff shows the wider surface |

**Moving** is `edit --move stock.items.reserve --to stock.holds`. Identity is the declaration's, not its path's, so tests, evidence and history follow, and the guarantee diff must be empty. When a slice passes the threshold in 26, the reply names the split that would cut it: states and laws that never meet.

## Context slices

A one-line change should not load a feature. In a public Ultimate example app the generated manifest is 52,501 tokens and the `posts` feature 60,808 of source plus 51,807 of tests (`o200k_base`, a proxy tokenizer).

```json
{"view":"slice","for":"accept","task":"edit-body","budget":2500,"have":["i:22"],
 "required":[{"h":"b:41","kind":"target"},{"h":"i:30","kind":"iface","of":"Invite"},
             {"h":"l:12","kind":"law"},{"h":"t:03","kind":"test"}],
 "optional":[{"h":"b:77","of":"revoke","why":"shares law","tokens":140}],
 "already":["i:22"],"fits":true}
```

- A body check depends only on the body, the interfaces it names, the laws whose state it touches, and its tests. So `required` **is the checker's input**: sufficient to check, not necessarily to understand.
- Across modules a slice holds **contracts only**. Checking never loads a neighbour's body; *running* a scenario across modules does, by hash, outside the context.
- Too big returns `fits:false` with the size needed. Nothing is dropped.
- `have` sends only what changed. Target goes last, which keeps a prompt-cache prefix stable.
- Derived plumbing (retries, pagination, caching, error mapping) is absent from a slice; `view --as policy` shows the named default in force.
- `editCost`, the size of `required`, is tracked per declaration.

## Many agents, one checkout

The declaration, not the file, is the unit of change. Each has a stable id, an interface hash and a body hash; a transaction carries the base hashes it edits **and the hashes it read**.

| Situation | Result |
|---|---|
| target changed since `base` | conflict |
| something read has changed | `E-STALE`; replayed automatically only if it rechecks with the same verdicts |
| both sides add fields, laws, cases | merged as keyed sets; laws conjoin |
| body edits on both sides | merged only if disjoint and tests pass on the result |
| delete against edit, rename against edit | always surfaces |
| anything proved that touched both sides | evidence dropped, re-earned |

The read set catches write skew: one agent relaxes `Invite` while another relies on it, with no shared line. Broken states live in a session overlay, so one agent's hole never reddens another's loop. A raw write past the daemon is recorded as unattributed: a lost update is detectable, not preventable.

## Intent

`@why` is indexed: reason, laws and tests that carry it, rules they constrain, generated files, monitors. `view why` walks it in either direction, from an outage line to the sentence someone wrote. Reports list reasons with no law and laws with no reason.

**Prose is tracked, not checked.** A reason is `unformalised`, `linked`, or `witnessed` (a person accepted concrete cases on both sides of the line). It is never "proven".

## Untrusted text

| Surface | Rule |
|---|---|
| opening, `view`, parsing, checking | executes nothing from the project; no build hooks |
| vocabulary packages | pure, terminating, resource-bounded expansion |
| `foreign` code | only under `test` or `build`, with named grants |
| comments, `@why`, strings from packages | omitted from slices; on request as `{"quoted":…,"origin":…}` |
| identifiers from packages | length-bounded, confusables rejected, can be aliased (`p1.f3`), because a name can carry an instruction |
| a package upgrade | a guarantee diff |

One context holding private data, untrusted text and an outward effect does not compile: split reader from actor ([05](05-effects-and-trust.md)).

## What a model is given

| Layer | Size | Loaded |
|---|---|---|
| [language card](01-language-card.md) | 4,868 tokens on 2026-11-24 (`o200k_base`), still changing | always |
| one vocabulary's card | goal 600 | on `use` |
| slice | measured per call | per edit |
| lessons | one form per error class | on error |

`view card` prints the card of the installed compiler. There is no training corpus. One can be generated: typed programs interpreted, mutated into failing pairs with counterexamples, and ported open-source functions kept when independent tests agree. None of it is shown to be enough ([19](19-density-and-experiments.md)).

## Limits

- The slice guarantee holds only while bodies check against interfaces. Every whole-program analysis added later (deadlock, worst-case time) breaks it and must say so.
- Prompt injection is reduced, not solved. Identifiers must be shown; `foreign` code can lie; covert channels exist.
- Semantic merge can accept two edits that are jointly wrong and untested.
- "Fixes already run" can overfit the tests that ran, and a small model will copy fix A. Copied fills are counted in [19](19-density-and-experiments.md).
- The synonym table and the third escalation are heuristics.
- Past some hundreds of modules the map must be paged by folder, and a wrong `@why` misleads search as a wrong comment does.
- Turn-time dominance is an expectation about current agents, not a measurement of this system.
