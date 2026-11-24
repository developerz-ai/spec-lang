# Inspirations

> **Design.** Nothing here runs yet. What spec-lang takes from other notations, the exact form it takes, and what it leaves behind.

**Admission rule.** An idea enters only if it passes both tests:

1. It makes logic easier for a model to **express** or easier to **check**.
2. It fits a form the [language card](01-language-card.md) already has: a type, a rule, a note, a test, a vocabulary kind, or a library function. The grammar does not grow for it.

An idea that passes 1 and fails 2 becomes a library, or nothing. In the tables, "Form" is card syntax unless it says *library*. Names in *library* and *vocabulary* cells are proposals. Rows without a source link rest on general knowledge and were not re-checked.

## Shells

| Source | Idea taken | Form | Not taken, and why |
|---|---|---|---|
| **PowerShell** | [Pipelines carry objects, not text](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_pipelines) | a typed chain: `Task.filter(not it.done).sort(it.due)` | `\|` as a second call syntax. Binding a piped object to whichever parameter matches by type or property name: the docs say you cannot force the choice. That is guessing |
| | [One approved verb per meaning; synonyms discouraged](https://learn.microsoft.com/en-us/powershell/scripting/developer/cmdlet/approved-verbs-for-windows-powershell-commands) (`Remove`, never `Delete`) | one verb per meaning in the standard library; six CLI verbs | the `Verb-Noun` spelling: the noun is already the type. Verb aliases |
| | [Validation attributes on parameters](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_functions_advanced_parameters) | the parameter's type: `attempts: 0..=10` | runtime-only checks. Defaults are not validated there, and `ValidateRange(0,10)` on a double accepts 10.5 after integer conversion |
| | [`-WhatIf`](https://learn.microsoft.com/en-us/powershell/scripting/learn/deep-dives/everything-about-shouldprocess) | a handler, below | opt-in per function |
| | `-Confirm` and impact levels | the guarantee diff at `land` ([08](08-approval-and-anti-cheating.md)) | prompts in the middle of a run, and `-Force` |
| **Bash** | composition of small parts in one line | calls and chains | text as the universal type. The [pitfalls list](https://mywiki.wooledge.org/BashPitfalls) has 65 entries and opens with word splitting |
| **jq** | a [path](https://jqlang.org/manual/) is a place you can read or update | `seats[n] = Held(by, until);` | filters with zero, one or many outputs: the multiplicity is invisible at the call |
| **Nushell** | [parse everything, then run; no `eval`](https://www.nushell.sh/book/thinking_in_nu.html); structured values throughout | whole-module checking before any step | implicit last-value return: the card requires `return` in a block |

**PowerShell's `-WhatIf`, made unavoidable.** `-WhatIf` is the best safety idea in any shell, and its weakness is documented: each author must opt in and wrap each change by hand, and the preference does not cross from one script module into another, which the linked guide calls "very dangerous". The cause is that effects are not in the signature. Here they are (`@uses`), so a dry run is a handler that records effects, and no rule can forget to support it:

```slg
@given(clock.at(0s), seats[1] -> Free, plan)
test hold_changes_one_seat = {
  hold(ana, 1);
  assert plan.writes.size == 1;
  assert seats[1].is(Held(ana, _));
}
```

`plan` is a *library* handler, proposed here. Tests are already a simulated world; the point is to run the same rule against **live** state and see the write set first, and the test pins what that set is. Writes go to an overlay, so later reads in the run see them, and the overlay is dropped. An effect whose reply nobody can know (a live payment provider, a `foreign` call) stops the plan as Blocked at that step, like a hole. A dry run that invents the reply would be a lie.

## General languages

| Source | Idea taken | Form | Not taken, and why |
|---|---|---|---|
| **Python** | examples beside the code ([doctest](https://docs.python.org/3/library/doctest.html)) | `@cases(5 -> 120)` | comparing printed text: one differing character fails, `1` matches `True`. Cases compare values |
| | comprehensions | `xs.filter(it > 0).map(it * 2)` | a second spelling of the same thing; indentation as structure |
| **Ruby** | dense chains; `it` as the unnamed argument | `seats.all(it.is(Free))` | [implicit last-expression return](https://docs.ruby-lang.org/en/master/syntax/methods_rdoc.html), monkey patching, optional parentheses |
| **RSpec** | [nested context, one fact per example](https://rspec.info/features/3-13/rspec-core/example-groups/basic-structure/) | `@given(ana_holds) test ...` continues a world | lazy `let`, shared contexts, a matcher language: long-distance meaning |
| **Cucumber** | given/when/then that a reader can approve | scenario tests that call rules directly | [step definitions](https://cucumber.io/docs/cucumber/step-definitions/): text matched to code by pattern. English is rendered from the test, never parsed |
| **Java annotations** | one uniform way to attach facts to a declaration | `@retry(3, on: Timeout) @pii` | open meaning: annotations ["have no direct effect"](https://docs.oracle.com/javase/tutorial/java/annotations/) on the code, so any tool may reinterpret them. Notes are a closed set with one typed meaning |
| **Haskell** | effects visible in types; [properties over generated inputs](https://hackage.haskell.org/package/QuickCheck) | `@uses(...)`; `test p(x: Int) = ...` | laziness, type-class overloading, user operators |
| **OCaml** | sums with checked matching | `type Seat = Free \| Held(...)`, `match` | functor machinery |
| **Elixir** | [patterns that bind](https://elixirschool.com/en/lessons/basics/pattern_matching) where they test | `@guard(seats[n].is(Held(by, until)) and now() < until else NotHeld)` | a match that raises; a pin operator. Refusal is a typed error |
| **Erlang** | [supervision as structure](https://www.erlang.org/doc/system/design_principles.html), apart from the work | *vocabulary*: a `service` with `@retry`, `@replicas` | untyped messages; "let it crash" as the only error model |
| **Clojure** | [identity, state and value are different things](https://clojure.org/about/state) | `state seats: ...` is a place; `let` is a value | macros (they are grammar growth), dynamic typing |
| **[Nim](https://nim-lang.org/docs/backends.html)** | one source compiled to C, C++, Objective-C or JavaScript, with calls in both directions | languages as build output; `foreign` in both directions; readable dense source | user macros that rewrite syntax: every project becomes a dialect a model has not seen |
| **Zig** | [no hidden control flow, no hidden allocation](https://ziglang.org/learn/overview/) | `@alloc(none)`, `@fails(...)`, `machine { }` | compile-time execution as the generics system |
| **Julia** | maths that reads like maths | `KP * e + KI * i`, `²`, `±` | multiple dispatch: [ambiguous calls raise an error at the call](https://docs.julialang.org/en/v1/manual/methods/). The card has no overloading |
| **Excel / Improv** | recalculation; [formulas name data instead of cell positions](https://en.wikipedia.org/wiki/Lotus_Improv) | `@live query` | cell coordinates and copied formulas: positional codes |

## Data and logic

| Source | Idea taken | Form | Not taken, and why |
|---|---|---|---|
| **SQL** | say which rows, not how to fetch them | `query mine(by: User) -> List[Task] = Task.filter(...).sort(...)` | null and three-valued logic. Without `ORDER BY` the order is [undefined](https://www.sqlite.org/lang_select.html); here a `List` from an unordered source is an `E-OPEN` question |
| **Datalog** | recursion over facts where [every query terminates and clause order is irrelevant](https://docs.racket-lang.org/datalog/Tutorial.html) | *library*: `closure(edges)` | a second evaluation model in the core |
| **Prolog** | a relation can run in several directions | *library* or solver, in tests only | backtracking in rules: cost and order become invisible |
| **[Lean 4](https://lean-lang.org/)** | `sorry` loads with a warning; goal state shows what is known and what is left; a small kernel rechecks every proof | `?hole` runs up to the gap and reports scope and need; `@assume` is always listed; a small trusted core ([10](10-compiler-and-ir.md)) | full dependent types and tactic proofs as daily work: models solve fewer Lean tasks than Dafny ones on published sets ([18](18-prior-art.md)) |
| **APL / BQN** | operate on whole collections; [rank says which axis](https://mlochbaum.github.io/BQN/doc/rank.html) | `.sum()`, `.map`, named axes as a *library* | one-glyph operators: rare in training text, and nameless programs are hard to edit |
| **TLA+** | a system is [states and allowed steps](https://lamport.azurewebsites.net/tla/high-level-view.html); claims quantify over behaviours | `rule`, `flow`, `law p(t: Trace[hold \| buy])` | a model separate from the program: it drifts. Primed variables become `old(x)` |
| **Scroll / Particles** | one node shape for everything, documents and data in [one line-per-node text](https://github.com/breck7/scrollsdk) | one declaration shape; `@why` beside the code | "[parse errors do not exist](https://ar5iv.labs.arxiv.org/html/1703.01192)": every text is valid, so a slip surfaces later, in meaning. We want the first error at the first wrong token. Nesting by indentation alone makes edits count spaces |
| **LaTeX** | write [content, not appearance](https://www.latex-project.org/about/) | `page` says structure; layout is a target's job | macros that rewrite the language |

A Python docstring example and a PowerShell range check, as types and cases:

```slg
@cases(0 -> 1, 5 -> 120)
fn factorial(n: 0..=20) -> Int = if n == 0 then 1 else n * factorial(n - 1);
```

## Sciences

| Source | Idea taken | Form | Not taken, and why |
|---|---|---|---|
| **Mathematics** | sets by condition, bounded quantifiers | `Addr where self mod 4KiB == 0`; `xs.all(p)`; `≤ ∈ ⇒` | juxtaposition as multiplication, overloaded symbols, implicit domains. `∀ ∃ ∑` are `.all`, `.any`, `sum` |
| **Physics** | every quantity has a dimension; results have tolerances ([checked, then erased](https://learn.microsoft.com/en-us/dotnet/fsharp/language-reference/units-of-measure) in F#) | `9.81[m/s²]`, `5m ± 0.1m` | dropping the unit "because it is obvious" |
| **Chemistry** | a reaction must balance | a conservation law over a trace | subscript notation; it is `[ ]` and numbers |
| **Biology** | [state local rules, derive the many species](https://en.wikipedia.org/wiki/Rule-based_modeling) | a `rule` over a pattern; the state space is explored, not listed | implicit random rates: randomness is an effect with a handler |
| **Control theory** | controller, plant and requirement are three things | `controller`, `plant`, `law settles(...)` | block diagrams as source; they are a view |
| **Modelica** | [equations state equality, not assignment](https://en.wikipedia.org/wiki/Modelica) | `der(w) = ...`, in a `plant` only | equations in ordinary rules: the order of effects would be unclear |

```slg
law atoms_conserved(t: Trace[react]) = t.steps.all(it.after.atoms == it.before.atoms);
```

## Institutions and arts

| Source | Idea taken | Form | Not taken, and why |
|---|---|---|---|
| **Accounting** | every entry [sums to zero](https://hledger.org/dev/hledger.html); corrections are new entries | a type that cannot hold an unbalanced entry | an amount left out and inferred: one visible rule, but it hides the mistake it exists to catch |
| **Law** | definitions, then rules with conditions and exceptions ([Catala](https://github.com/CatalaLang/catala) builds on default logic) | `type` for a defined term, `match` with every case, `@why` for the source text | implicit priority between exceptions: overlap is an error or an `E-OPEN` question |
| **Linguistics** | one abstract tree, [many renderings](https://en.wikipedia.org/wiki/Grammatical_Framework_(programming_language)) | `view`: English, diagram, target code | parsing natural language into source |
| **Music** | a score names repetition and can [write it out in full](https://lilypond.org/doc/v2.24/Documentation/notation/long-repeats) | shorthand plus `view --as expanded`; `@on(tick(60Hz))` as tempo | position on a staff as meaning |
| **Chess** | a whole [position](https://www.chessprogramming.org/Forsyth-Edwards_Notation) in one line, a whole [game](https://en.wikipedia.org/wiki/Portable_Game_Notation) as replayable moves | `@given(world)` plus a `Trace` is a replayable bug report, when every effect is modelled | positional letters and digit run-lengths: counting |

```slg
type Posting(account: Id[Account], amount: Money[USD]);
type Entry = List[Posting] where self.sum(it.amount) == 0USD;
```

## What the list shows

| Pattern | Seen in | Rule we keep |
|---|---|---|
| The safety net works when it is not optional | `-WhatIf`, types against validation attributes | put it in the signature |
| Text as an interface rots | Bash, Cucumber, doctest | values in, values out; text is a rendering |
| Position as meaning fails under edits | spreadsheets, FEN, indentation | names |
| Invisible priority is the bug | overload resolution, legal exceptions, row order | overlap is an error or a question |
| Conservation is the cheapest strong check | accounting, chemistry, physics | a `law` before an example |

## Limits

- The admission rule is a judgement until measured. Each row should be traceable to a blind-author result in [19](19-density-and-experiments.md); today none is.
- `plan` depends on a richer `handler` than the one-line form the card shows, and on a way to point one at live state, which the six verbs do not yet name. Planning a long-running flow (waits, retries) is unsolved: the overlay is only right if nothing else writes meanwhile.
- "A `List` from an unordered source is `E-OPEN`" follows from *minimal and sufficient* but is not written in the card. It may ask too often.
- Catala's exceptions are more expressive than `match` for real statutes. A tax vocabulary may need its own kind.
- Rows on Haskell's types, OCaml, Prolog, LaTeX's macros, chemistry and mathematics are unsourced background.
