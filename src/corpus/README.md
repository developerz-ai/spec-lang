# The corpus

> **Status `As of 2026-11-24`.** 38 programs, each green under `spec-lang test <program> --deny warnings`, and a judge, `src/bin/corpus`, that runs them. The engine is the only side that exists: the LIR evaluator and every printer report `not built`.

What "any program" means for the compiler of [decision 009](../../docs/decisions/009-one-compiler-any-language.md): the programs whose every check the LIR evaluator and each printer's output must report as the engine does. The engine says what a program means; the corpus says which programs.

## The judge

```bash
src/bin/corpus                 # every program here, plus pocs/todo-app/src and pocs/platformer/src
src/bin/corpus f05-int-mod     # some programs
```

For each program it collects one status per check from each side and compares them with the engine's:

| Side | Command | Status |
|---|---|---|
| engine | `spec-lang test <p> --json` (`--deny warnings` for the programs here) | runs |
| lir | `spec-lang lir <p> --eval --json` | not built |
| `rust` `ts` `ruby` `cpp` | `spec-lang build <p> --target T --tests --out <tmp>`, then `<tmp>/bin/test` | not built |

**The contract of a side:** one JSON object per line and per check, `{"check": "<name>", "status": "pass" | "fail" | "blocked"}`; other fields and lines that are not such objects are ignored. A check's name is its declaration path (`tank.laws.a_refusal_changes_nothing`) for a test, property or law, and `<path> case[<i>]` for row `i`, from 0 in written order, of a `@cases` table (`ints.pow case[4]`): the names `spec-lang lir --eval` prints. The judge derives the engine's names from `kind` and `decl` of `spec-lang test --json`. The exit code of `bin/test` is not read.

A cell is `agree`, `DISAGREE <check>: engine <status>, <side> <status>` (the first differing check in the engine's order; a check one side lacks is `missing`), `error` (the side exists and printed no check: a build that failed, a runner that crashed; its first lines are printed under the table), or `not built` (the command or flag is unknown: exit 2). The run exits 1 on a `DISAGREE` or an `error`, or when the engine is not green on a program. Programs run in parallel; the table is in name order and holds no timing, so two runs print the same bytes. A new target is one word in the `targets` line of the script.

## Feature programs

One per feature of the card the engine runs. Each file starts with a comment that says what it pins; this table is the index.

| Program | Checks | Pins |
|---|---|---|
| `f01-records` | 21 | construction, defaults (also computed), nested records, `.with` as a copy, field-by-field equality |
| `f02-sums` | 32 | sums with and without data, `match` with literal, binding, wildcard and nested patterns, first arm wins, `.is` |
| `f03-ranges-claims` | 28 | range types, `T where claim` on Int, Text and records, generated values inside every claim, a guard refusing before a narrow place is written |
| `f04-int-edges` | 45 | exact `+ - * ^` past 2^53, 2^63 and 2^64 up to 2^127 - 1, sign and precedence, hex and `_` literals |
| `f05-int-mod` | 54 | floored `mod` for every sign pair and past a word, `mod 0` fails `DivByZero`, `abs min max clamp` |
| `f06-bits` | 59 | `band bor bxor shl shr` past 32 and 64 bits, shift counts beyond the word, their precedence |
| `f07-text-size` | 57 | `size` in code points, `bytes` in UTF-8, `take` and `drop` by code point, no normalisation, `<` by code point |
| `f08-text-trim` | 21 | `trim` removes exactly the 25 `White_Space` code points; the code points where host languages differ |
| `f09-text-interpolation` | 33 | `"{expr}"`, Text raw and data as source (defaults printed, sets and maps ascending), the six escapes |
| `f10-text-ops` | 72 | `split join lower starts_with contains replace`, empty pieces, final sigma, a `lower` that grows |
| `f11-list-access` | 55 | `size first last get [i] contains find position take drop reversed +`, `push insert remove`, `Missing` |
| `f12-list-transform` | 50 | `filter map flat_map fold sum min max all any`, order kept, ties to the earliest, empty lists |
| `f13-list-order` | 34 | stable `sort`, `desc: true` against `.reversed()`, `distinct`, `zip` and `Uneven`, `group_by` |
| `f14-set` | 29 | one of each, ascending iteration, `+ -`, `insert` and `remove` quiet, the function names on a Set |
| `f15-map` | 26 | ascending keys, `Missing` and `else`, `m[k] = v`, `m[k].f = v`, record and tuple keys, maps in maps |
| `f16-option-else` | 30 | `Some` and `none`, `x else v` on a failure and on an Option, `else fail E`, the right side run only when needed |
| `f17-equality-order` | 16 | structural `==`; the one order of Bool, Int, Text, records, sums, lists, tuples, options, sets |
| `f18-lambdas` | 22 | `it`, named and two-argument lambdas, block bodies, captured locals, parameters and state, nesting |
| `f19-loops-var-return` | 51 | `let var for while`, the looped collection read once, `return` from nested loops, `return;` in a rule, recursion |
| `f20-conditions-logic` | 58 | `if then else` as a value, `and or not implies` precedence and short circuit, chained comparisons, `in` |
| `f21-tuples` | 30 | tuples as values, keys and sort keys, `match` on a tuple, nested tuples, `()` |
| `f22-range-values` | 52 | `a..=b`, `a..<b`, empty when reversed, `size list map filter all any sum fold`, bounds past a word |
| `f23-rule-guards` | 10 | guards in written order across and inside notes, a failing condition gives its guard's error, replies |
| `f24-rule-atomic` | 11 | a failure mid-body undoes every write and drops the buffered `out.line` lines; earlier calls stay |
| `f25-rules-in-rules` | 11 | an inner refusal fails and undoes the outer rule, three levels deep; `inner() else fail E`; a handled guard refusal |
| `f26-state-ownership` | 9 | three modules: state written only by its own rules, read across modules, one atomic move over two modules |
| `f27-given-chains` | 9 | `@given(state -> value)`, `@given(test)` four deep, siblings apart, a parent then places on top |
| `f28-scenarios` | 5 | `call;`, `call -> value;`, `call -> fail E;` for guards, `fail`, `Missing`, `Uneven`, `DivByZero` |
| `f29-properties` | 14 | generation from every type form, early return, a fresh world per run, `@given` and `@cases` on a test |
| `f30-laws-traces` | 15 | `t.states`, `t.steps` with `before after event reply`, `s.event.n`, `s.event.is(rule(..))`, event sets, a law parameter |
| `f31-foreign` | 19 (7 blocked) | a check that reaches a `foreign` body is blocked, one that does not runs; guards first; blocked parents and laws |
| `f32-effects-out` | 7 | `out.line`: one `\n` per call, call order, raw text, nothing from a refused call |
| `f33-modules-consts` | 23 | path is the module, `use` of a module and of a declaration, a sum's variants, constants of every shape |

Expected values in the number and text programs were computed outside the engine (Python's integers, `str` methods and Unicode data) before they were written down. Where the engine and that oracle differed the row was wrong, never the engine.

## Ports of the design POCs

The five programs of `docs/pocs/*/program.md`, kept to the part the engine runs. Each has a `README.md` with its cut table: the form as the design writes it, the diagnostic the engine gives for it, and what the port does instead. A modelled form (time as a state moved by a rule, `@access` as a first guard) is marked as a model there.

| Program | Checks | Kept | Cut |
|---|---|---|---|
| [`p01-seat-limits`](p01-seat-limits/README.md) | 8 | four rules, two laws, the six scenarios | `Id[T]`, `@access`, `Map.entries`, `@given(m[k] -> v)` |
| [`p02-ledger`](p02-ledger/README.md) | 6 | `transfer`, two laws, three scenarios, the property | `Id[T]`, `@idempotent` and the test `sent_again` |
| [`p03-invites-and-webhooks`](p03-invites-and-webhooks/README.md) | 8 | three rules, two laws, six scenarios | `flow` and `@edge`, `event` and `.emit`, `@access`, `@secret` on a parameter, `random.token()`, the clock, `crash`, two tests |
| [`p04-task-lifecycle`](p04-task-lifecycle/README.md) | 10 | four rules, the two safety laws, the four scenarios | `Time`, units, `now()`, `@on(every(1s))`, a binding `.is`, `crash`, `t.within` |
| [`p05-elevator`](p05-elevator/README.md) | 13 | `step`, two rules, one law, two scenarios | `@on(tick(1Hz))`, the clock, `t.within` |

### The cut list

Every form the ports had to leave out, once. This is the to-do list of the engine and of the compiler: a port gets its form back when the row goes.

| Form | The engine says | Ports |
|---|---|---|
| `Id[Org]` | `E-UNSUPPORTED` unsupported: the type `Id[Org]` | p01 to p04 |
| `Id(1)` | `E-NAME-UNKNOWN` no `Id` in scope | p01 to p04 |
| `@access(..)` on a rule | `E-UNSUPPORTED` unsupported: the note `@access` on a `rule` | p01, p03 |
| `@idempotent(request)` | `E-UNSUPPORTED` unsupported: the note `@idempotent` on a `rule` | p02 |
| `flow` with `@edge` | `E-UNSUPPORTED` unsupported: kind `flow` | p03 |
| `event`, `accepted.emit(..)` | `E-UNSUPPORTED` unsupported: kind `event`; `E-NAME-UNKNOWN` no `accepted` in scope | p03 |
| `@secret token: Text` on a rule parameter | `E-UNSUPPORTED` unsupported: notes on parameters | p03 |
| `random.token()` | `E-NAME-UNKNOWN` no `random` in scope | p03 |
| `Time` | `E-NAME-UNKNOWN` no `Time` in scope | p04 |
| `5min`, `7d`, `40s`, `1s` | `E-UNSUPPORTED` unsupported: the number `5min`: only whole numbers without a unit | p03, p04, p05 |
| `now()` | `E-NAME-UNKNOWN` no `now` in scope | p04 |
| `@on(every(1s))` | `E-UNSUPPORTED` unsupported: the trigger `every(1s)` | p04 |
| `@on(tick(1Hz))` in a module without `use game` | `E-UNSUPPORTED` unsupported: the trigger `tick(1Hz)` | p05 |
| `@given(clock.at(0s))` | `E-UNSUPPORTED` unsupported: the `@given` item `clock.at(0s)` (handlers are not implemented) | p03, p04, p05 |
| `clock.advance(..)` | `E-NAME-UNKNOWN` no `clock` in scope | p03, p04, p05 |
| `@given(m[k] -> v)` | `E-UNSUPPORTED` unsupported: the `@given` item `orgs[1] -> Org(limit: 3)` | p01 to p04 |
| `crash` in `Trace[..]` | `E-NAME-UNKNOWN` no `crash` in scope | p03, p04 |
| `t.within(d, p)` | `E-UNSUPPORTED` unsupported: `t.within`: liveness over traces, when the law runs | p04, p05 |
| `x.is(Running(box, until))`, binding | `E-UNSUPPORTED` unsupported: a pattern in `.is(...)` that binds names | p04 |
| `m.entries` | `E-TYPE-MISMATCH` `Map(..)` has no `entries`, when a check reaches it | p01, p04 |

## Engine findings

Found while writing the corpus. None is fixed here, and no program pins the behaviour in question.

1. **A handled failure of an inner rule keeps the inner rule's writes.** A rule that writes and then fails, called from another rule as `inner(..) else v`, leaves its writes and its `out.line` lines in the outer rule's world. The same call from a test (`assert (inner(5) else -1) == -1;`) undoes them. The card says a rule's writes happen all or none.

   ```slg
   use cli;
   state log: List[Text];
   rule inner(x: Int) -> Int = {
     log.push("try {x}");
     out.line("trying {x}");
     if x > 2 { fail TooBig; }
     return x;
   }
   rule outer(x: Int) -> Int = {
     let r = inner(x) else -1;
     log.push("done {r}");
     return r;
   }
   test kept = {
     outer(5) -> -1;
     assert log == ["done -1"];
   }
   ```

   The assert is false: `log` is `["try 5", "done -1"]` and `out.text` is `"trying 5\n"`.
2. **`Ok(v)` cannot be written with a value.** `assert Ok(1) == Ok(1);` stops with `E-CLAIM`: field `value` is not a (): 1 is not (). So `s.reply == Ok(s.after.level)` cannot be asked; `f30-laws-traces` compares the printed reply (`"{s.reply}" == "Ok({s.after.level})"`).
3. **A law with a second parameter is judged on a start world that breaks it.** The start world is accepted as lawful for one value of the parameter and the trace is then judged with another, so the law is "false after 0 call(s) from a random lawful world". Reproduction and the workaround (quantify inside the law) are in [`p02-ledger/README.md`](p02-ledger/README.md).
4. **`type Change = up | down;` in the module that declares the rules `up` and `down` is `E-NAME-AMBIG`** ("up is declared twice"): the names are read as new variants. The card's form works from another module (`f30-laws-traces`).
5. **Missing forms that are not `E-UNSUPPORTED`.** `try_call(..)`, `a.div(b, floor)`, `now()`, `Id(1)`, `random.token()`, `clock.advance(..)` give `E-NAME-UNKNOWN`; `m.entries` gives `E-TYPE-MISMATCH` and `t.within` its `E-UNSUPPORTED` only when a check reaches them. `5.clamp(7, 3)` (bounds the wrong way round) says `5 has no clamp`.
6. **Stops short of the engine's own bound.** `0 shl 500` stops with `E-RUNTIME` (an Int beyond 128 bits) though the answer is 0; `(0..=18446744073709551616).size` stops with `E-RUNTIME` (a range of more than ten million numbers) though `x in` that range answers. Neither is a wrong answer.
7. **A text literal cannot hold `"` inside `{...}`.** `"{xs.join(",")}"` is a parse error: the grammar's `TEXT` token ends at the inner quote. This is the grammar, not the engine; the workaround is a `let`.

## Limits

- **Only the engine has run these programs.** No LIR evaluator and no printer exists yet, so the judge has compared nothing real. Its `agree`, `DISAGREE`, `missing`, `error` and `not built` cells were exercised once by hand with a stand-in binary that altered one status, dropped one check and added one.
- **A green corpus cannot hold a failing check, so three behaviours are not pinned:** a broken claim (`E-CLAIM` is a failed check, not a refusal a test can expect), an overflow past the engine's `Int` (`E-RUNTIME`), and a compile-time refusal such as writing another module's state (`E-STATE-FOREIGN`). A backend that does not monitor claims passes every program here.
- **Properties and laws that pass say little about the generator.** The judge compares statuses, not generated values: a backend with another generator agrees wherever the claim is true for every value. Only a failing property or law would show that seeds differ.
- **Check names are a convention**, shared with `crates/lir` (`lower/checks.rs`) and written nowhere in `docs/`; each printer's test runner has to meet it.
- **No mutation run on the feature programs.** That their tests "would catch a wrong translation" rests on exact expected values and on rows chosen where host languages are known to differ, not on a measured kill rate. The ports' READMEs list the mutants tried on them, once, by hand.
- **Six feature programs exceed 120 lines** (`f19-loops-var-return` is 185).
- **The ports are smaller than their designs**: the cut list above is what they lack, and a modelled clock or access check is not the form it stands for.
- The expected values from Python depend on its Unicode version for `lower` and on nothing else; the 25 white-space code points were written from the Unicode property list, not from Python's `isspace` (which differs).
