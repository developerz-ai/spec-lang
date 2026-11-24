# Density and experiments

> **Design.** Nothing here runs yet. No experiment below has been run. What "concise and sufficient" means, how to measure it, and the outcomes that stop the project.

**Rule.** The metric must be able to say that the language lost. Two halves, and neither may buy the other: fewer tokens per accepted change, and **zero undecided behaviours**.

## Three metrics that look right and are not

| Metric | Why it fails |
|---|---|
| source tokens or lines | rewards hiding code in a package, dropping a check, or leaving a behaviour to accident |
| behaviour per source token | "behaviour" has no unit; whoever counts it decides the winner |
| pass rate on small functions | models are near the ceiling there in familiar languages ([18](18-prior-art.md)) |

**Static size is a diagnostic only.** It is reported, never ranked, and never in a headline. One measurement shows why (`o200k_base`, a proxy; counts say nothing about accuracy): [`seats.slg`](24-examples.md#seats-pure-logic-scenarios-laws) is 578 tokens, and the [language card](01-language-card.md) is 4,884 (2026-11-24, still changing). The card is as long as about eight such modules, before the first one is written. One program cannot pay for a language; a year of changes might.

## Half one: cost per accepted change

```text
cost = (uncached-equivalent input + output tokens, all turns, all attempts, failures included) / accepted changes
```

| Counted | Note |
|---|---|
| card, vocabulary cards, package text, slices, every diagnostic | no free documentation |
| failed attempts, rereads, rework after review | on the accepting denominator |
| generator scripts an agent writes to avoid the language | agents [do this](https://arxiv.org/abs/2606.10933) in unfamiliar languages; the share is reported |
| cached input | a second figure at provider prices, never the first |

Beside it, always: escaped defects by severity, first-correct time, human review minutes and errors, verification and build time, generated-code performance, dollars. None is folded in.

## Half two: sufficiency

A change is **admissible** only if it leaves no behaviour undecided. There are two kinds, and the compiler sees one.

| Kind | Example | Found by |
|---|---|---|
| **open**: the language allows two behaviours | a derived body with two solutions; a tie in a promised order; a retried step that is not safe to repeat | `E-OPEN`, from the closed list of choice points ([07](07-agent-loop.md)). Must be 0 |
| **accidental**: the language picks one, and nothing anyone wrote or was told picked it | who else may call it; what happens to runs in flight at a release | probes, below. Must be 0 |

The second matters most. Any deterministic language scores 0 on the first. "One behaviour" is not "a decided behaviour".

**Probes.** For each task, adjudicators who wrote no arm list concrete situations (input, world, fault) whose answer a user, operator or attacker could observe, and mark which the requirement settles and which it leaves open. A probe counts as decided in an arm when:

1. a law, test, type or written rule fails if the behaviour at the probe is flipped, **or**
2. it follows from a named default that the arm's own documentation states, and two blind readers given only source and documentation predict it, **or**
3. the arm raised it as a question before landing.

```text
undecided(change) = E-OPEN + probes decided by none of 1, 2, 3
accepted(change)  = acceptance suite green and undecided == 0 and nothing weakened without its owner
```

Condition 2 charges hidden convention. Condition 3 rewards asking. For requirement-open probes only 3, or a later human decision, counts: picking one silently is a failure even when the pick is sensible. The same probes and rules apply to every arm, so TypeScript is not failed for lacking `E-OPEN`.

Static density can now be stated without cheating, as a diagnostic:

```text
SD = probes decided by 1 / (source tokens + package tokens amortised over its users)
```

## The arms

| Arm | What | Isolates |
|---|---|---|
| A | idiomatic Ultimate, TypeScript, its own gate | today |
| B | A plus equal effort: tuned docs, property tests, fault simulation, the same agent protocol where it fits | better tooling, no new semantics |
| C | spec-lang | the proposal |
| **D** | **the same semantics embedded in TypeScript**: same IR, interpreter, server, diagnostics, tests | **whether a language is needed** |
| E | spec-lang IR with longer, more familiar spelling | the notation choices |
| F | Dafny, pure-logic tasks only | whether an existing verified language already does it |

**D is the decisive control.** C against A shows only that something helped. If D matches C, the value is the checker and the loop, and the right product is a library in a language models already know. Wasp went that way ([18](18-prior-art.md)). D gets the same polish as C.

## E0 comes before the compiler

| Piece | Cost-cutting choice |
|---|---|
| semantics | D's library and interpreter only |
| C | the parser (exists for this card) and a thin lowering to D's IR, or hand translation, stated per task |
| tasks | 12, pilot only, outside the sealed set |
| asked of models | author from a requirement; edit; predict a reply; find the wrong line; fill a hole from its failing tests |
| asked of people | spot a planted weakening from a projection and from a PR diff |
| blind authors | fresh sessions given the card and nothing else; the card is fixed where they fail, at most twice |

E0 can only stop the project or continue it. It cannot confirm the thesis.

## The sealed protocol

| Choice | Rule |
|---|---|
| tasks | 60, sealed, power-analysed first: features, requirement changes, bugs, migrations, two-target logic, brownfield calls, concurrent-edit pairs, contradictory specifications, injected text, and some at machine altitude where we expect no gain |
| chains | a fifth are 10-change sequences on one growing program; endpoint is change 10 |
| corpus ports | [corpus](16-corpus.md) cases 1, 4, 6 with their existing tests as black-box oracles; count what had to be added back |
| models | three families, each at a frontier and a small tier, clean sessions, k = 5, random order, no step or token caps |
| frozen | compiler, cards, prompts, per phase |
| separation | language authors write no tasks, probes or graders; training or example material shares no repository with the tasks |
| graders | code first; a judge only where code cannot see, calibrated on human labels |
| statistics | paired, bootstrap over tasks and chains, not over runs |
| published | all traces, failures included |

Primary endpoints, named beforehand: pass^3, severity-weighted escapes, cost at change 10, undecided behaviours. All else is secondary.

## Small models, measured

The claim: compiler, types and errors carry enough of the intelligence that a cheap model is productive. It is an arm, crossed with B, C and D.

| Sub-arm | Who writes what |
|---|---|
| S0 | frontier writes everything |
| S1 | small writes everything, with the card and the error loop |
| **S2** | **frontier writes types, guards, laws, tests and holes; small fills the holes** |
| S3 | S2, with errors reduced to code and location |

S2 against S0 is the price of intelligence. S2 against S3 is what the error contract is worth ([07](07-agent-loop.md)). S2 in C against S2 in D is whether the language adds to it. The frontier's tokens for laws and tests, and every escalation, are in S2's cost. Endpoints: hole-fill pass^3, whole-task pass^3, cost per accepted change, escalations per task, and the share of fills copied unchanged from a suggested edit, because a fill the compiler wrote is not a model being productive.

**Gaming trials** run in each arm: contradictory tasks, a visible law that blocks the easy fix, a package with instructions in names and comments. Correct outcomes include a dispute or a question. The counted failures: cheat accepted, guarantee weakened unseen, honest task refused.

## Kill criteria

Thresholds are proposals until frozen after power analysis. Each names a decision, not a regret.

| ID | If the sealed result is | Then |
|---|---|---|
| K1 | upper 95% bound of C/B cost at change 10 is not below 0.80 | drop every token claim |
| K2 | C does not cut severity-weighted escapes by 30% against B at non-inferior pass^3 (margin 5 points) | drop the guarantee claim |
| **K3** | **upper bound of C/D cost is above 0.90, or no quality gain over D** | **ship D; stop the language** |
| K4 | on two of three families C's pass^3 is more than 5 points under B after two card revisions | stop general-purpose use |
| K5 | reviewers miss a planted critical weakening, or do worse than with a PR diff | no approval without source |
| K6 | p95 edit to first counterexample over 10 s on modules, or over 20% of risky claims never leave `assumed` | narrow the domains claimed |
| K7 | over 40% of a pilot's source is `foreign` or glue | drop breadth for that domain |
| K8 | accepted-cheat rate not below B's, or one bypass of authority | no unattended landing |
| K9 | C leaves more undecided behaviours than D or B on the same probes | conciseness came from omission: fix or stop |
| K10 | under half of production tokens disappear on the corpus ports with behaviour kept | drop the plumbing claim |
| K11 | S2 whole-task pass^3 is more than 10 points under S0, or S2 is not cheaper per accepted change with escalations counted | drop the small-model claim |

K1 and K2 can both fail while a useful checker survives. K3 ends the language. K9 is the one a concise design is most likely to fail quietly, which is why it is separate from cost.

## Evidence we start from

| Finding | Reads as | Caution |
|---|---|---|
| [GPT-5 on Idris](https://arxiv.org/abs/2602.11481): 22 of 56 zero-shot, 54 of 56 with compiler-error-guided refinement | feedback can carry an uncommon language | one model, exercise-sized; Idris is not zero-data |
| [Vericoding](https://arxiv.org/abs/2509.22908): 27% Lean, 44% Verus, 82% Dafny, off-the-shelf models | writing to a given specification is possible, and the language matters | different problem sets per language |
| [AlgoVeri](https://arxiv.org/html/2602.09464v1): Gemini 3 Flash 40.3% Dafny, 24.7% Verus, 7.8% Lean, on aligned problems | harder, aligned tasks are far lower | one model's figures quoted |
| [Type-Error Ablation](https://arxiv.org/html/2606.01522v1): Claude Haiku 4.5, 92.8% untyped to 95.7% with full error detail | at the frontier, detail buys a little | 33.5% untyped to 52.8% on a small open model |

None of these tests a language like this one, at repository scale, over a year.

## Limits

- **Probes are people's imagination.** Undecided behaviours outside the list are invisible to every arm equally.
- Condition 2 depends on reader skill; agreement between readers is reported.
- Sixty tasks from one organisation's world do not generalise; another organisation's set is needed before any public claim.
- B and D can be under-built by accident; equal time is only attested.
- Models change under the experiment. Results carry a date and expire.
- K5 bounds its sample; it never shows a miss rate of zero.
