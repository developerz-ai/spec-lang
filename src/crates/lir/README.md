# spec-lang-lir

> **Status `As of 2026-11-24`.** The LIR of [decision 009](../../../docs/decisions/009-one-compiler-any-language.md): `lower` turns every program the engine runs into one typed, language-neutral tree; each `app` gets a manifest; an evaluator of the LIR runs every check. On the 40 programs of `src/corpus` and the two POCs, `spec-lang lir <p> --eval --json` reports what `spec-lang test <p> --json` reports, check by check (`src/bin/corpus`). No printer reads the LIR yet.

## Run it

```bash
target/release/spec-lang lir ../pocs/todo-app/src                 # the LIR as text
target/release/spec-lang lir ../pocs/todo-app/src --json          # one JSON document, for tools and printers
target/release/spec-lang lir ../pocs/platformer/src --eval --json # one {"check", "status", "diagnostic"} per check
SPEC_LANG=$PWD/target/release/spec-lang bin/corpus                 # the judge: engine against LIR, every program
```

`--eval` takes `test`'s options (`--seed --runs --traces --steps --jobs`); its output does not depend on `--jobs`. A program must pass the check stage first; a form the LIR cannot say is an error at its span, and nothing runs.

## The LIR in one screen

```text
Lir      modules · types · claims · consts · states · fns · rules · handlers · tests · laws · checks · apps
         (ids are the IR's: a TypeId, FnId, RuleId ... names the same declaration in both)
TypeDecl Alias{of: DTy} | Record{fields} | Sum{variants} | RuleSet{rules}     field = name, LTy, check DTy, default Expr?
Claim    self: LTy -> Bool body (`T where p`, one per written claim; DTy::Where points at it)
State    LTy, check DTy, init Expr? (absent: empty Map/Set/List/none)
Func     params (name, LTy, check DTy, default Expr?), result LTy + check, body Expr | Extern{lang, library, symbol, effects}
Rule     as Func, plus guards [(cond, Error)] in order and triggers (library notes as data)
Handler  a library's operations (out.line, out.text) and its pretend: state + one arm per operation, run by tests
Test     params, given [state := Expr | end world of Test], body Block
Law      params, which is the trace, traced rules, start rules, body Expr
Check    name, kind (case | test | property | law), what: Case{target, args, expect} | Test | Property | Law
Manifest per app: target, shape, notes, params naming declarations, every reachable rule

Expr     every node has its LTy
  literals · Interp[Lit | Show(e)] · Local · Const · State · Variant{ty, tag} · Some · None
  Construct{ty, tag?, args: [Expr?]} · CallFn · CallRule · Effect{handler, op, name, args} · EffectRead
  Prim{op, args} · Lambda{params, captures, result, body} · CallLambda
  Field{index} · VariantField{by_tag} · WorldState{state} · EventArg{by_rule} · With{changes}
  Is{value, Pat} · If · Logic{and|or|implies} · Compare{first, links} · Else{unwrap} · Fail{name, args}
  Match{arms: (Pat, Expr)} · Block · Hole · Tuple · List · SetOf · MapOf · Range
Stmt     Let{check?} · Assign{Place} · Mutate{Place, Prim} · Expr · Return · Assert · For(over a List) · While · If · Expect
Place    root State | Local, steps Field{index} | MapKey(e) | ListIndex(e), the value's LTy, check DTy?
Pat      steps run in order: Tag{path, ty, tag} | Event{path, rule} | Equals{path, e} | Never | Bind{name, path, LTy}
         path = [VariantField{ty, tag, index} | TupleItem(i) | EventArg{rule, index}] from the scrutinee
```

Two kinds of type. `LTy` is every expression's: claims erased (an alias is its base, a range type `Int`), records and sums nominal, plus `Range`, `Func`, `Event[R]`, `Reply[R]`, `Failure`, `Step[R]`, `World`, `Trace[R]` for traces, and `Never` for `fail` and `?hole`. `DTy` is a declared type kept where a value is checked against it: ranges and `where` claims included.

## What each node means

The engine is the authority; these are its semantics, written so a printer can follow them.

| Rule | Meaning |
|---|---|
| Order | left to right: a call's arguments, then the callee; a primitive's arguments in its signature's order; `Construct` evaluates the given arguments, then per field in order its default (evaluated in no scope) when absent, then `conform` |
| `conform(v, DTy)` | the base first, then the claim (`true` holds; `false`, a failure or a non-`Bool` does not); collections item by item (a map's keys, then its values); records and variants field by field; `Option`'s payload; `Result` and builtin sums are not looked into. Broken: stops `E-CLAIM` |
| Where `conform` runs | every argument after defaults are filled (a `fn`'s and a rule's prologue), every result, every constructed field, every `.with` change, a typed `let`, every state write (`Place.check`; a whole collection after `push`/`insert`/`remove`), every `@given` state value, every state's initial value, a handler operation's arguments. A `const` is not checked at run time (the check stage does) |
| `fn` | prologue: absent arguments take their default with the earlier parameters in scope; then `conform` each; an `Extern` body blocks (`B-FOREIGN`); else the body; then `conform` the result |
| rule | prologue as `fn`; guards in order: `false`, or a failure while evaluating the condition, fails with that guard's error; then the body. **Atomic**: on any halt, the world and every handler's pretend state go back to what they were before the call, for a rule called inside a rule as well (its failure, when the caller handles it with `else`, undoes only its own writes); otherwise an inner rule joins the caller's step |
| `Effect` | the handler's pretend arm runs: its state and the arguments are locals, the body runs, the state's locals as they end are the new state; the body's value is the reply. A runtime does the real thing (a line on standard output) |
| Failure | `fail E(data)`, `Missing` (absent key or index, empty `first`/`min`), `DivByZero`, `Uneven`: handled by `else`, a rule's refusal, a test's `-> fail E`. Anything else halts the check |
| `Else{unwrap}` | a failure of the value gives the fallback; with `unwrap`, `none` too, and `Some(v)` gives `v`. `unwrap` is false on `m[k] else v`, so a stored `none` stays |
| Blocks | a `Block` expression's value is its `return`, or `()` when it ends; a `return` inside `if`/`for`/`while` leaves the innermost `Block` expression (a body, a lambda, a match arm's block) |
| `For` | its list is evaluated once, before the loop (a `Set` iterates through `set.list`, a range through `range.list`) |
| Lambdas | captured locals are copied when the lambda is made |
| `Compare` | a link is `op(left, right)`, except `x in c`, which is `op(c, x)`; each right side runs only while every earlier link held |
| Patterns | steps in order; the first failing test is no match; bindings do not escape an `Is` |
| Show | `"{x}"` prints Text raw and anything else as source: `src/eval/show.rs` is the format, character for character |
| Value order | `false < true`; numbers by value; Text by code point (a prefix first); records field by field; sums by declared variant, then fields; lists and tuples item by item; sets and maps as their ascending sequences. `Set` and `Map` iterate ascending; every `sort`, `min`, `max`, `distinct`, `group_by` uses this order |
| Int | exact. A printer may use any representation that stops with `E-RUNTIME` where the exact result leaves it: the engine and the evaluator use `i128` with checked `+ - * neg pow shl abs`, and every corpus program stays inside it. 64 bits are not enough: `f04-int-edges` computes up to `2^127 - 1` |
| Text | sizes, `take` and `drop` count code points; `trim` removes the 25 `White_Space` code points; nothing is normalised |
| Budget | 20,000,000 evaluation steps per machine, then `E-RUNTIME` (the engine's; the counts are not identical, so a program that runs out may disagree) |

## Primitives

93, each with one meaning. "fails E" is a language failure; "stops CODE" halts the check.

| Primitive | Signature | Meaning |
|---|---|---|
| `eq` | `(T, T) -> Bool` | structural equality on every type but functions |
| `ne` | `(T, T) -> Bool` | not `eq` |
| `not` | `(Bool) -> Bool` | negation |
| `int.add` | `(Int, Int) -> Int` | exact sum; stops E-RUNTIME outside the printer's integer range (the engine's is i128) |
| `int.sub` | `(Int, Int) -> Int` | exact difference; stops E-RUNTIME on overflow |
| `int.mul` | `(Int, Int) -> Int` | exact product; stops E-RUNTIME on overflow |
| `int.neg` | `(Int) -> Int` | negation; stops E-RUNTIME on overflow |
| `int.mod` | `(Int, Int) -> Int` | floored modulo: the result has the sign of the divisor (`-1 mod 4 == 3`, `1 mod -4 == -3`); divisor 0 fails DivByZero |
| `int.pow` | `(Int, Int) -> Int` | exact power; a negative exponent stops E-TYPE-MISMATCH; overflow stops E-RUNTIME |
| `int.div` | `(Int, Int) -> Int` | always stops E-OPEN: `/` on Int is an open choice (write `a.div(b, floor)`) |
| `int.band` | `(Int, Int) -> Int` | bitwise and; a negative operand stops E-TYPE-MISMATCH |
| `int.bor` | `(Int, Int) -> Int` | bitwise or; a negative operand stops E-TYPE-MISMATCH |
| `int.bxor` | `(Int, Int) -> Int` | bitwise xor; a negative operand stops E-TYPE-MISMATCH |
| `int.shl` | `(Int, Int) -> Int` | a * 2^b; a negative operand stops E-TYPE-MISMATCH; overflow stops E-RUNTIME |
| `int.shr` | `(Int, Int) -> Int` | floor(a / 2^b), 0 when b >= 128; a negative operand stops E-TYPE-MISMATCH |
| `int.lt` | `(Int, Int) -> Bool` | a < b |
| `int.le` | `(Int, Int) -> Bool` | a <= b |
| `int.gt` | `(Int, Int) -> Bool` | a > b |
| `int.ge` | `(Int, Int) -> Bool` | a >= b |
| `int.abs` | `(Int) -> Int` | absolute value; stops E-TYPE-MISMATCH when it does not fit |
| `int.min` | `(Int, Int) -> Int` | the smaller |
| `int.max` | `(Int, Int) -> Int` | the larger |
| `int.clamp` | `(Int, lo: Int, hi: Int) -> Int` | the value pulled into lo..=hi; lo > hi stops E-TYPE-MISMATCH |
| `text.lt` | `(Text, Text) -> Bool` | code point order; a prefix comes first |
| `text.le` | `(Text, Text) -> Bool` | code point order |
| `text.gt` | `(Text, Text) -> Bool` | code point order |
| `text.ge` | `(Text, Text) -> Bool` | code point order |
| `text.size` | `(Text) -> Int` | number of code points |
| `text.bytes` | `(Text) -> List[Int]` | the UTF-8 bytes, each 0..=255 |
| `text.trim` | `(Text) -> Text` | removes Unicode White_Space (25 code points; not U+200B, U+FEFF, U+180E) from both ends |
| `text.lower` | `(Text) -> Text` | full Unicode lowercase mapping, no locale |
| `text.starts_with` | `(Text, Text) -> Bool` | prefix test |
| `text.contains` | `(Text, Text) -> Bool` | substring test (also `x in t`) |
| `text.split` | `(Text, sep: Text) -> List[Text]` | pieces between separators, empty ones kept; an empty separator stops E-TYPE-ARITY |
| `text.replace` | `(Text, old: Text, new: Text) -> Text` | every non-overlapping match, left to right; an empty `old` stops E-TYPE-ARITY |
| `text.take` | `(Text, Int) -> Text` | the first n code points (a negative n is 0) |
| `text.drop` | `(Text, Int) -> Text` | all but the first n code points (a negative n is 0) |
| `list.size` | `(List[T]) -> Int` | number of items |
| `list.first` | `(List[T]) -> T` | the first item; empty fails Missing |
| `list.last` | `(List[T]) -> T` | the last item; empty fails Missing |
| `list.get` | `(List[T], Int) -> Option[T]` | the item at an index from 0, or none |
| `list.index` | `(List[T], Int) -> T` | `xs[i]`: the item at i; outside fails Missing |
| `list.filter` | `(List[T], T -> Bool) -> List[T]` | the items the predicate keeps, in order |
| `list.map` | `(List[T], T -> U) -> List[U]` | f of every item, in order; the first failure wins |
| `list.flat_map` | `(List[T], T -> List[U]) -> List[U]` | the lists f gives, joined in order |
| `list.fold` | `(List[T], A, (A, T) -> A) -> A` | left to right from the start value |
| `list.all` | `(List[T], T -> Bool) -> Bool` | every item holds; stops at the first false; empty is true |
| `list.any` | `(List[T], T -> Bool) -> Bool` | some item holds; stops at the first true; empty is false |
| `list.find` | `(List[T], T -> Bool) -> Option[T]` | the earliest item that holds |
| `list.position` | `(List[T], T -> Bool) -> Option[Int]` | the index of the earliest item that holds |
| `list.sum` | `(List[Int]) -> Int` | the sum, 0 when empty; overflow stops E-RUNTIME |
| `list.sum_by` | `(List[T], T -> Int) -> Int` | the sum of f of every item |
| `list.min` | `(List[T]) -> T` | the least item in the value order, the earliest of ties; empty fails Missing |
| `list.max` | `(List[T]) -> T` | the greatest item, the earliest of ties; empty fails Missing |
| `list.min_by` | `(List[T], T -> K) -> T` | the item with the least key, the earliest of ties; empty fails Missing |
| `list.max_by` | `(List[T], T -> K) -> T` | the item with the greatest key, the earliest of ties; empty fails Missing |
| `list.contains` | `(List[T], T) -> Bool` | some item equals x |
| `list.distinct` | `(List[T]) -> List[T]` | the earliest of each group of equal items, in order |
| `list.sort` | `(List[T], desc: Bool) -> List[T]` | stable sort by the value order; desc reverses the order, not the ties |
| `list.sort_by` | `(List[T], T -> K, desc: Bool) -> List[T]` | stable sort by the key's value order; desc reverses the order, not the ties |
| `list.reversed` | `(List[T]) -> List[T]` | the items in reverse order |
| `list.take` | `(List[T], Int) -> List[T]` | the first n items (all when fewer); a negative n stops E-TYPE-ARITY |
| `list.drop` | `(List[T], Int) -> List[T]` | all but the first n items; a negative n stops E-TYPE-ARITY |
| `list.zip` | `(List[T], List[U]) -> List[(T, U)]` | pairs by position; unequal sizes fail Uneven |
| `list.group_by` | `(List[T], T -> K) -> Map[K, List[T]]` | items by key, order kept inside a group |
| `list.join` | `(List[Text], Text) -> Text` | the texts with the separator between |
| `list.concat` | `(List[T], List[T]) -> List[T]` | `a + b`: a then b |
| `list.push` | `(List[T], T) -> List[T]` | the list with x at the end |
| `list.insert` | `(List[T], Int, T) -> List[T]` | x put before index at (0..=size); outside fails Missing |
| `list.remove` | `(List[T], Int) -> List[T]` | the item at index removed; outside fails Missing |
| `set.list` | `(Set[T]) -> List[T]` | the elements in ascending value order |
| `set.of_list` | `(List[T]) -> Set[T]` | the set of the items |
| `set.size` | `(Set[T]) -> Int` | number of elements |
| `set.contains` | `(Set[T], T) -> Bool` | membership (also `x in s`) |
| `set.union` | `(Set[T], Set[T]) -> Set[T]` | `a + b` |
| `set.difference` | `(Set[T], Set[T]) -> Set[T]` | `a - b` |
| `set.insert` | `(Set[T], T) -> Set[T]` | the set with x; already there: unchanged |
| `set.remove` | `(Set[T], T) -> Set[T]` | the set without x; absent: unchanged |
| `map.size` | `(Map[K, V]) -> Int` | number of entries |
| `map.keys` | `(Map[K, V]) -> List[K]` | keys ascending |
| `map.values` | `(Map[K, V]) -> List[V]` | values in ascending key order |
| `map.get` | `(Map[K, V], K) -> Option[V]` | the value at k, or none |
| `map.index` | `(Map[K, V], K) -> V` | `m[k]`: the value at k; absent fails Missing |
| `map.has` | `(Map[K, V], K) -> Bool` | `k in m`: k is a key |
| `map.remove` | `(Map[K, V], K) -> Map[K, V]` | the map without k; absent: unchanged |
| `range.list` | `(Range) -> List[Int]` | the numbers lo..=hi (or lo..<hi) ascending; more than ten million stops E-RUNTIME |
| `range.has` | `(Range, Int) -> Bool` | `x in lo..=hi` |
| `trace.steps` | `(Trace[R]) -> List[Step[R]]` | the steps in order |
| `trace.states` | `(Trace[R]) -> List[World]` | the start world, then the world after each step |
| `step.event` | `(Step[R]) -> Event[R]` | the call: its rule and arguments |
| `step.reply` | `(Step[R]) -> Reply[R]` | Ok(value) or Err(failure); a refused step has before == after |
| `step.before` | `(Step[R]) -> World` | the world before the call |
| `step.after` | `(Step[R]) -> World` | the world after the call |


## Tests as data

| Check | LIR | Run as |
|---|---|---|
| `@cases` row | `Case{target: Fn | Test, args (None: default), expect}` | a fresh machine, the arguments in no scope, one call; a refusal is compared by name only. On a test: its `@given` world, then the test with the row's values |
| test | `Test` id | `@given` items in order (a state set after `conform`; a parent test's end world, run once and remembered; a parent that failed or reached a hole blocks it `F-BLOCKED`, one that reached a foreign body blocks it `B-FOREIGN`); then the steps. Handler state starts fresh in every test |
| property | `Property` id; parameters carry their `DTy` | `runs` runs; run `i` draws every parameter from `seed_for(seed, "<test path>", i)` with empty pools, on a fresh machine; the lowest-numbered run that does not pass decides |
| law | `Law` id; traced rules, start rules | trace `i` from `Rng(seed_for(seed, "<law path>", i))`: odd `i` first draws a lawful world (every state from its `DTy`, up to 200 tries, kept when every law of the program holds on it alone with its other parameters drawn); even `i`, or no lawful world, starts at the initial world and makes `below(prefix + 1)` random calls of the start rules; then `steps` random calls of the traced rules; then the law's other parameters with pools of every world of the trace. A call picks a rule with `pick`, then its arguments with pools of the current world |

**Generation** (`src/eval/rng.rs`, `generate.rs`): SplitMix64; `seed_for` is FNV-1a of the check's path, mixed with the base seed and the run number, then one SplitMix64 step. A value of a `DTy`: when a pool holds values of exactly that type, half the time one of them (`chance(1, 2)`, `pick`); else a base value; kept when it conforms, else drawn again (400 tries, then `E-GEN`). Base values: `Bool` `chance(1, 2)`; `Int` by `below(100)` bands (0..=10, 11..=100, -10..=-1, eleven word edges, 101..=2^62, -(11..=2^62)); `Text` from one of four alphabets; a range one of its ends one time in five, else uniform; `Option` `none` one time in three; `List`, `Set`, `Map` `below(4)` items; records field by field; sums `below(variants)`. Pools hold the keys of each state declared `Map[K, V]` and the elements of each declared `Set[K]`.

**What is reproducible.** Everything above is a pure function of the seed, so a printer that implements the generator draws the engine's inputs exactly: the same property arguments, the same traces and lawful worlds; the evaluator does, which is why a law it finds false is false on the engine's same trace number. What is not: shrinking (the engine shrinks a failure; the evaluator reports the failing run unshrunk, so messages differ, statuses do not), and the evaluation budget's count.

## The manifest

`Manifest` per app: `name`, `qualified`, `module`, `target` (`@target`), `shape` (the library whose app shape it has, `game`), `notes` (each library note as `{library, name, args: [{param, value}]}`, values `Int | Text | Quantity{value, unit} | Variant{name, args}`), `params` (`rules: [...]`, then the shape's parameters, each naming rules or `fn`s), and `rules`: every rule a named declaration reaches through calls. The JSON spells each rule out: parameters with `LTy`, `DTy` and default, result, triggers (library notes, `cli("add")`, `tick(60Hz)`), `why`, `uses`, and its `extern` when foreign. Nothing in this crate knows a library by name.

## A new printer, checklist

1. Types: a record per `Record`, a tagged union per `Sum` (`Option` and trace replies included), aliases as their base with a checked constructor; `conform` per `DTy`, claims as functions of `self`.
2. A runtime: exact `Int` that stops on overflow, Text by code point with the 25-point `trim`, ordered `Set` and `Map` with the value order, structural equality, `show`.
3. The 93 primitives above, each once.
4. Every `ExprKind`, `StmtKind`, `Place` and `PatStep`, in the order "What each node means" gives.
5. Rules: the prologue, guards in order, the snapshot of world and handler state, restore on failure.
6. Each library's handler: the runtime's real operation for the app, the pretend for tests; `Extern` calls bound to `library#symbol` in `libs/<library>/real/<lang>`.
7. Tests: every `Check` as a native test named `check.name`, printing `{"check", "status"}` lines; the generator and seeds above for properties and laws; `blocked` for a check that reaches an `Extern`.
8. The app: read the manifest; the library runtime does the rest.
9. Judge it: add the target to `src/bin/corpus` and make every row agree.

## Not lowered

Each is an error at its span, never a silent answer: everything the engine refuses (`src/README.md`); building `Ok(..)` or `Err(..)` as a value, and `Some` or `none` used without their form (`E-UNSUPPORTED`; the engine refuses `Ok(1)` with `E-CLAIM`, see `src/corpus/README.md`); a declaration whose type depends on itself without a written result type; a member of a value whose type is not known where it is used; a state name shared by two modules read from a world; `==` on functions; a chained comparison across two types; the machine-only operators `own`, `ref`, `mut`. A mistyped expression is `E-TYPE-MISMATCH` at lowering, where the engine finds it only on a path a test reaches.

## Where the LIR differs from the engine

1. **A handled inner refusal.** `inner() else v` inside a rule: the LIR undoes the inner rule's writes and lines (card, How it runs 1); the engine keeps them (`src/corpus/README.md`, engine finding 1). No program of the corpus pins it; `tests/lower.rs` pins the LIR's.
2. **Types decided before running.** Where the engine decides by the value (`a else b else c` with `a` and `b` Options; `==` between different types), the LIR gives each expression one type: the first is lowered as `a else (b else c)`, which gives the same value on every input; the second is a block that evaluates both sides and gives `false` (`true` for `!=`), as the value order does. A collection whose items nothing types (`[] + [] == []`) has items of type `()`.
3. **Pretend arms** are lowered as ordinary code; a printer's test runtime runs them as the engine does.

## Measured

Method: `spec-lang test` and `spec-lang lir --eval` on the same program, release build, default counts, 16 hardware threads, one run each (`time`). Caveat: one machine, no load controlled, one run.

| Program | Engine wall | LIR evaluator wall | Agreement |
|---|---|---|---|
| `pocs/todo-app/src` (35 checks) | 0.18 s | 0.21 s | 35 of 35 |
| `pocs/platformer/src` (90 checks) | 3.1 s | 4.0 s | 90 of 90 (89 pass, 1 blocked) |
| `src/corpus`, 38 programs | not timed | not timed | every check of every program |

Two mutants of the todo program (`next_no = no`, a misspelt `!high`) fail the same checks on both sides, law included.

## Limits

- The judge is agreement with the engine, not correctness: where the engine is wrong, the LIR is wrong the same way, except where this README says otherwise.
- Types are inferred per declaration by unification; there is no checker of the card's subtyping or claims at lowering, only what conform checks at run time.
- Statuses agree; diagnostics do not (messages, spans, shrunk counterexamples differ).
- `Int` is `i128` in the evaluator, like the engine; the language's is unbounded.
- The evaluation budget counts differently from the engine's.
- Only the programs of `src/corpus` and the two POCs were judged; another program may find a lowering the corpus does not.
- No printer exists; the checklist above is untested by one.
