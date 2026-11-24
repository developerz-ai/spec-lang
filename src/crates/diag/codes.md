# Diagnostic codes

One line per code: what it means and the rule it enforces. A code is stable: tools and agents match on it. `docs/idea/01-language-card.md` is "the card"; `30` is `docs/idea/30-type-rules.md`.

## Parse

| Code | Means | Enforces |
|---|---|---|
| `E-PARSE` | the text is not in the grammar | `docs/idea/25-grammar.md` |
| `E-PARSE-LEX` | a character or text literal no terminal matches | `25`, terminals |
| `E-LOAD` | a file or directory cannot be read, or two files make one module | card, Names: the path is the module |

## Check

| Code | Means | Enforces |
|---|---|---|
| `E-NAME-UNKNOWN` | a name resolves to nothing | `30` section 2 |
| `E-NAME-AMBIG` | a name is declared twice, or bound both here and by `use` | `30` section 2 |
| `E-NAME-SHADOW` | a local reuses a name already in scope, or a body calls a core free name its local has taken | card, Expressions: no shadowing; core free names |
| `E-NAME-PRIVATE` | a name of another module that is not `pub` (tests and states excepted) | `30` section 2 |
| `E-NAME-KIND` | a name is used as something it is not (a type as a value, a rule as a type, `fail(...)` as a pattern) | `30` section 2 |
| `E-USE-CYCLE` | modules `use` each other in a cycle | card, Names: `use` has no cycles |
| `E-KIND-UNKNOWN` | a declaration's kind is neither core nor from a used vocabulary | card, One declaration shape |
| `E-NOTE-UNKNOWN` | a note that no core rule or used vocabulary defines | card, Core notes: an unknown note is an error |
| `E-NOTE-PLACE` | a known note on a kind it does not belong to | card, Core notes: a misplaced note is an error |
| `E-NOTE-ARG` | a note's arguments have the wrong form | `30` section 9 |
| `E-NOTE-DUP` | a note that may appear once appears twice | `30` section 9, N-Once |
| `E-TYPE-ARITY` | a call or case gives too many, too few, unknown or repeated arguments | `30` section 4, T-Call |
| `E-TYPE-MISMATCH` | a value is not of the type written | `30` section 3 |
| `E-STATE-FOREIGN` | a write to state of another module, or outside a rule, or a `@given` setting another module's state | card, Names: a module owns its state |
| `E-PLACE-ASSIGN` | an assignment to something that is not a `var` or a `state` | `30` section 7 |
| `E-EFFECT-UNDECLARED` | a `pub` rule's body has an effect its `@uses` does not list | `30` section 8 |
| `E-EFFECT-KIND` | an effect where the kind forbids it (`out.line` in a `fn`) | `30` section 8, E-Kind |
| `E-GUARD-FORM` | a `@guard` clause is not `condition else Error` | card, Core notes: `@guard(p else Err)` |
| `E-UNSUPPORTED` | a form this toolchain does not implement yet; never a silent wrong answer | `src/README.md`, Limits |
| `E-OPEN` | two behaviours remain; the diagnostic lists both as alternatives | card, Minimal and sufficient |

## Lint (warnings; errors under `--deny warnings`)

| Code | Means | Enforces |
|---|---|---|
| `W-LINT-WHY` | a `pub` rule without `@why` | card, Core notes: `@why` is the reason |
| `W-LINT-UNUSED-USE` | a `use` whose name is never used | `34-hygiene.md`: nothing unused |
| `W-LINT-UNUSED-DECL` | a non-`pub` `fn`, `type`, `const` or `state` nothing refers to | `34-hygiene.md` |
| `W-LINT-EFFECT-UNUSED` | `@uses` lists an effect the body never has | `30` section 8 |
| `W-LINT-NO-EXPECT` | a test with no `assert` and no `->` step (and not used by `@given`) | card, Tests |
| `W-LINT-LAW-INERT` | a law's trace names a rule that writes no state the law reads | `06-logic-and-tests.md`: a law checks what rules change |

## Test

| Code | Means | Enforces |
|---|---|---|
| `F-CASE` | a `@cases` row gave another value or failure | card, Tests: table |
| `F-TEST` | a scenario step, `assert` or expectation did not hold | card, Tests: scenario |
| `F-PROPERTY` | a property test failed for a generated input (shrunk) | card, Tests: property |
| `F-LAW` | a law was false on a generated trace (shrunk counterexample; says whether it started from a reached or a random lawful world, and offers a missing state law when one fits) | card, How it runs 2; Traces |
| `F-BLOCKED` | a test did not run: its `@given` parent failed, or it reached a `?hole` | `06-logic-and-tests.md` |
| `E-CLAIM` | a value broke a `where` claim: an argument, a field, a state write or a `@given` | `30` section 5, O-Sub |
| `E-RUNTIME` | evaluation stopped: a type error found by running, an overflow beyond the engine's integers, a step budget | `src/README.md`, Limits |
| `E-GEN` | no value of a parameter's type could be generated | `06-logic-and-tests.md`: tight claims are an error, not a skip |
