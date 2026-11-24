# Approval and anti-cheating

> **Design.** Nothing here runs yet. How a person approves a change without reading source, and why an agent cannot go green by weakening the promise.

**Rule.** People approve behaviour and guarantees, shown as things that can be run or counted. They are never asked to read source, and never shown prose a model wrote about its own change.

## Projections

All are rendered by the compiler from one IR hash.

| Projection | Why it can be believed | Use |
|---|---|---|
| **witness diff** | each line is executable before and after | behaviour |
| **guarantee diff** | covers a fixed list of dimensions; unknowns shown | every change |
| decision table | parses back to the same IR on finite partitions | rules, access |
| scenario English | one template per step, from the calls ([06](06-logic-and-tests.md)) | stories |
| state or topology diagram | exact graph, with node and edge counts | flows, infrastructure |
| pseudo-code, TypeScript, Rust | explanatory, marks what it omits | curiosity, never approval |

Author text (`@why`, names) is quoted, attributed and set apart. A risky predicate is expanded to its definition, because a rule called `is_safe` can say anything. The renderer joins the trusted base: money, access and deletion use two independent renderers plus the table round trip.

## The guarantee diff

```json
{"base":"g:aa","head":"g:bf","why":["a lost reply must never charge twice"],
 "stronger":[{"law":"paid_once","how":"new implies old","by":"solver"}],
 "weaker":[{"rule":"refund","what":"guard","was":"age <= 90d","now":"age <= 30d",
   "witness":{"age":"31d","before":"ok","after":"fail Expired"}}],
 "unknown":[{"law":"fair_queue","shownAs":"weaker"}],
 "newlyAssumed":[{"fact":"gateway ids are unique","in":"settle"}],
 "evidence":[{"law":"conserved","from":"proven","to":"sampled"}],
 "effects":{"added":["http.call"],"removed":[]},
 "access":{"refund":{"gained":["support of the same tenant"],"lost":[],"witness":"by: #9 support"}},
 "defaults":[{"policy":"retry","was":3,"now":5,"cause":"compiler upgrade"}],
 "data":{"irreversible":[]},
 "behaviour":{"changed":3,"noDifferenceFound":2,"notSearched":0},
 "open":[{"ask":"partial refund, then dispute","A":"keep","B":"reverse","codeDoes":"A"}]}
```

Dimensions: accepted inputs, replies, laws, time bounds, effects, access, information flow, data, dependencies, budgets, evidence, assumptions, derived defaults, targets.

| Rule | Reason |
|---|---|
| Direction is decided by implication, with variance | a stricter `@guard` is a *weaker* service; a stricter `@post` is a stronger one |
| Undecided means weaker | unknown must cost attention |
| A changed default is a change | behaviour can move with no source edit |
| A lost level is listed even when every test passes | evidence is part of the promise |

## Approving

1. **See witnesses.** For each changed behaviour, the smallest input on which old and new differ: `refund at 31 days: ok -> fail Expired`. If search finds none it says "no difference found in N runs", never "unchanged".
2. **Answer open questions.** The compiler searches for *another* body that satisfies every law and test yet differs on some input, and asks: "both are allowed, the code does A; intended?" The answer lands as a test with a human origin. Questions are ranked by money, access, privacy and deletion; an unanswered high-risk one blocks.
3. **Sign.** The signature covers base, head, guarantee diff, renderer version and the list of unanswered questions. Any change to them voids it. New assumptions, wider access and irreversible data are signed one by one.

A change with equal guarantees, unchanged authority and no witness found can land by policy, with no person. What nobody modelled (looks, desirability, a missing requirement) needs the running preview, and no diff will show it.

## The threat

It is measured, not imagined. In [ImpossibleBench](https://arxiv.org/html/2510.20270v1), tasks whose tests contradict the specification can only be passed by cheating, and frontier models often do.

| Their finding | Taken here |
|---|---|
| Letting the model stop and flag the conflict cut GPT-5 from 54% to 9% and o3 from 49% to 12% (Conflicting-SWEbench, full scaffold); much weaker for Claude Opus 4.1 | a legal exit: `land --dispute` |
| Read-only tests restored honest performance and stopped test edits (no figure in the text) | laws and human tests are visible, and protected at landing |
| Hidden tests brought cheating near zero but hurt honest performance | do not hide the specification; add seeded checks at landing |

## Moves and counters

| Move to go green | Counter |
|---|---|
| delete or edit a law, test or case | classified by meaning at `land`, whatever the text diff looks like; needs the law's owner |
| narrow the domain: a new `@guard`, a tighter type, a smaller range | appears under `weaker` with a witness |
| make a law vacuous: a premise no allowed world satisfies, an empty type, laws that contradict each other | `E-VACUOUS`: every premise needs a witness, every type an inhabitant, every clause a mutant it rejects |
| shrink where traces start: add a convenient state law, a flow edge removed, a tight `@given` on the law | a state law must hold of the initial world and be kept by **every** rule of the module, or it is refuted; it lands as a law, with an origin, and other laws list it under what they rest on |
| hide a reachable bad start as "none found" | that label is search, `bounded(k)`; the law stays red until the missing law holds |
| special-case the example's literal | a branch on a constant that occurs only in a test is `E-SPECIAL`, shown with perturbed inputs; a signed exception clears it |
| move logic to `foreign` or `machine` | the claims resting on it drop a level, listed |
| widen `@access`, drop one in favour of a `@guard` on `by`, make a subject `fn` looser | `@access` is the only access mechanism: no clause is denied, a guard on `by` grants nothing, and the diff shows a caller who gained |
| expose a rule by adding it to an `app` | access dimension: callers before and after |
| raise `@within`, stretch `@retry`, drop a monitor | `weaker` |
| change a handler so the world is kinder | handlers are protected evidence; conformance must re-pass |
| write laws that match the bug | same-session law, test and body carry one origin and count once |
| add `@assume` | `newlyAssumed`; needs the assumption owner, signed singly |
| weaken a dependency | an upgrade is a guarantee diff |
| change the authority policy | `land` uses the policy of the **base** commit |

## Origin

Every law, test and case has an origin: person, incident, import, session. Independence is counted per origin, not per test. A session may write as many tests as it likes to think with; for landing, tests it wrote with the body add no independent evidence, and the report says how many remain.

## Mutating both ways

| Mutate | Expect | A survivor means |
|---|---|---|
| the body | some law or test fails | a weak specification: it becomes a question |
| the law | some accepted or rejected witness contradicts it | a law that pins nothing |

Mutants are derived from the IR with the declaration hash plus a landing nonce, recorded so the run replays. Results are killed, surviving, equivalent, unresolved; there is no percentage target, because a target is the next thing to game. Equivalence is undecidable. The method is known to find real problems: [IronSpec](https://www.usenix.org/conference/osdi24/presentation/goldweber) reports ten specification bugs across six verified systems using specification testing that includes mutation, and [MutDafny](https://arxiv.org/abs/2511.15403) found five specifications needing strengthening among 794 Dafny programs. Neither says it finds most.

## A dispute is a success

```json
{"dispute":"sold_is_final","replay":"c:88",
 "conflict":["law: a sold seat never changes","test: refund_frees_seat expects Free"],
 "resolutions":[{"edit":"law exempts refunded seats","guarantee":"weakens"},
                {"edit":"drop refund_frees_seat","guarantee":"weakens"}],
 "state":"open"}
```

It never turns red into green. It ends the agent's task with a replay, the contradiction and options, and routes to whoever owns the law. The agent is measured on a correct dispute exactly as on a correct fix.

## Who may change what

```json
{"policy":"v3","roles":{
  "builder":["body","test.add","law.add","law.strengthen","dispute"],
  "law-owner":["law.relax","test.retire"],
  "risk-owner":["assume.add","access.widen","effect.add","foreign.add"]},
 "rules":[{"op":"law.relax","not":"author of a body in the same change"},
          {"op":"data.irreversible","needs":["risk-owner","person"]}]}
```

Locally an agent may edit anything, with no prompts. The fence is at `land`, which runs in a protected verifier, with keys absent from the agent's environment, against the base commit's policy. Roles attach to the meaning of an operation, never to a path.

## Is the approver any good?

Projections are tested like code: plant critical weakenings in practice changes, and measure how many a reviewer catches from the projection and from an ordinary PR diff. The catch rate is the projection's own evidence. If reviewers miss a planted critical weakening, or do worse than with the diff, approval without source is withdrawn (K5 in [19](19-density-and-experiments.md)). Practice changes are labelled as such afterwards and never land.

## Limits

- **Specification independence is the weakest point of the whole design.** Two origins can share one wrong idea. Witnesses and mutants expose what was left open, never what nobody imagined.
- Search for a distinguishing input is incomplete. "No difference found" is a sample.
- Question fatigue is real. Ranking can bury the one that mattered, and an approver who always answers A is a rubber stamp the system cannot detect.
- Vacuity, special-casing and equivalence checks are heuristics at the edges; a patient adversary can blend a special case into plausible logic.
- Nothing here binds someone who controls the protected verifier.
