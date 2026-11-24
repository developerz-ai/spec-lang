# Standard library

> **Design.** Nothing here runs yet. Every name in the core library: its signature, how it fails, what it means, and the choices a backend may not make differently.

A name neither here nor on the [card](01-language-card.md) comes from a vocabulary ([29](29-first-vocabularies.md)). **new**: not on the card yet.

## How to read the tables

1. A cell `find(p: T -> Bool) -> Option[T]` is `pub fn find(...) -> Option[T];` inside its type. Call it on a value: `xs.find(it > 3)`.
2. **No parentheses in the signature, none in the call**: `xs.size`, but `xs.reversed()`.
3. Names belong to types, so `Text.size` and `List.size` are two functions, not an overload. `min(a, b)` is an error whose edit is `a.min(b)`. The only free names are `now`, `try_call`, `all`, `race`, `timeout` and, in `@post`, `old`.
4. Everything is pure unless a row gives an effect. Pure names never change their receiver.
5. **Three verbs change a place**: `push`, `insert`, `remove`. Each is a statement on a `var`, `state` or `mut` place, returns nothing, and has the effect `place.write`. Machine altitude adds `store`, `replace_if` and `write`, under the same rule.
6. **Fails** is the complete list. A dash means total.

## One rule for absence

| Question asked | Absent answer | Examples |
|---|---|---|
| "give me the thing that should be there" | fails `Missing` | `m[k]` `xs[i]` `first` `last` `min()` `max()` |
| "is there one?" | `none` | `get` `find` `position` |

`x else v` handles both: where `x` can fail, it handles the failure; otherwise, where `x` is an `Option`, it unwraps. On `m: Map[K, Option[V]]`, `m[k] else v` therefore handles a missing key and leaves a stored `none` alone. A failure neither excluded by a claim nor handled fails the rule with that error ([03](03-types-and-claims.md)).

**In `@access`, `Missing` counts as denied.** `@access(Task[id].owner == by)` on an id that does not exist gives the denial error, `NotFound` by default, in a direct test call too; so a missing row and somebody else's row answer alike.

## Decisions

| Question | Answer | Reason |
|---|---|---|
| Map and Set iteration | ascending key order, always | equal values must iterate equally, on every backend |
| order of any data | `false < true`; numbers by value; `Text` by code point; records field by field; sums by declared variant, then fields; lists and tuples item by item | any value except a function or `Float` can be a key |
| `<` in source | numbers, `Time`, `Text`, vocabulary dates | sort keys use the order above |
| sort | stable, both directions | `xs.sort(k).reversed()` also reverses ties; `desc: true` does not |
| ties in `min(f)`, `max(f)` | the earliest | one answer |
| index | from 0; no negative index; outside fails `Missing` | |
| `take(n)` `drop(n)` past the end | everything, nothing | the size is a limit, not a claim |
| `zip` of unequal sizes | fails `Uneven` | truncation hides bugs |
| `for x in e` | `e` is evaluated once, before the loop | removing while iterating is safe |
| what `Text.size` counts | Unicode scalar values (code points) | does not change with the Unicode version, unlike grapheme clusters |
| `Text` equality | same code points; nothing is normalised | |
| `and` `or` `implies` | left to right, stop when decided; so do chained comparisons | `k in m and m[k] > 0` is safe |
| `Int` overflow | cannot happen; a backend proves a width from claims or uses a big integer | |
| value outside a range type | an obligation, never a wrap or a clamp | `floor = floor + 1` must be shown to stay a `Floor` |
| `u8 ... i64` overflow | wraps | layouts only |
| division | `a.div(b, mode)`; `b == 0` fails `DivByZero`; `Int / Int` is `E-OPEN` | |
| number literals | take the type the context wants: `10` can be `Decimal`; a value never converts by itself | |
| `+` **new** | `List + List` appends; `Set + Set`, `Set - Set` are union and difference; nothing on `Text` (write `"{a}{b}"`) or `Map` | |
| `in` and `contains` | same meaning; the formatter prints `x in xs` | one spelling in storage |

## Bool, Option, Result

| Signature | Fails | Means |
|---|---|---|
| `is(p: Pattern) -> Bool` | - | on any value: does it match? May bind names ([patterns](#patterns)) |
| `with(f: Fields) -> Self` | - | on any record: a copy with some fields changed |
| `try_call(x: Call) -> Result[T]` | - | `Ok(v)` or `Err(e)`; the failed call's writes are undone |

`Option[T]` is `Some(v) | none`; `Result[T]` is `Ok(v) | Err(e)`. Both are ordinary sums used with `match`, `is` and `else`.

## Int

| Signature | Fails | Means |
|---|---|---|
| `abs() -> Int` | - | |
| `min(b: Int) -> Int` | - | the smaller; `max` likewise |
| `clamp(lo: Int, hi: Int) -> Int` | - | needs `lo <= hi`, an obligation |
| `div(b: Int, mode: Rounding) -> Int` | `DivByZero` | exact quotient, rounded |
| `decimal: Decimal` | - | **new** |
| `float: Float` | - | **new**; nearest, ties to even |

`a mod b` fails `DivByZero`, and equals `a - b * a.div(b, floor)`: with `b > 0` it is in `0..<b`, also for negative `a`. `a ^ n` needs `n >= 0`. `band bor bxor shl shr` need non-negative `Int`.

**Rounding**: `floor ceil trunc away half_even half_away`. There is no `half_up`: libraries disagree on negative ties, so the error offers `half_away` and `ceil`.

## u8 ... u64, i8 ... i64

| Signature | Fails | Means |
|---|---|---|
| `wrap(n: Int) -> Self` | - | **new**, on the type: `u8.wrap(300) == 44` |
| `int: Int` | - | **new**; exact |

`u8(n)` is an obligation that `n` fits. `+ - *` wrap; `div` and `mod` as for `Int`. `shl` and `shr` need a count below the width; `shr` keeps the sign of `i` types. Mixing widths is an error.

## Decimal, Money

`Decimal[s]` has `s` digits after the point. `Money[C]` is a decimal at its currency's scale, from a currency table pinned in the lock file.

| Signature | Fails | Means |
|---|---|---|
| `round(mode: Rounding) -> Self` | - | back to the declared scale |
| `div(b: Self, mode: Rounding) -> Int` | `DivByZero` | whole times `b` fits |
| `abs() -> Self` | - | `min max clamp` as for `Int` |
| `split(n: Int) -> List[Self]` | `DivByZero` | **new**; parts sum exactly, earlier parts take the spare units |

`+ -` are exact. `*` by a `Decimal` or a percent is exact and **unrounded**: such a value can be compared or rounded, not stored or returned as `Money`. `/` is `E-OPEN`. `Money * Money` and two currencies do not type-check.

## Float

`+ - * /` and square root are correctly rounded ([05](05-effects-and-trust.md)).

| Signature | Fails | Means |
|---|---|---|
| `round(mode: Rounding) -> Int` | `NotFinite` | dimensionless only: `(x / 1m).round(floor)` |
| `abs() -> Self` | - | `min max clamp` as for `Int` |

NaN equals itself and orders after every number, and `-0.0 == 0.0`. Not IEEE: it keeps `==` structural and `sort` total, and a backend pays where a NaN is possible. `Float` is not a `Map` key or `Set` element. `sin`, `exp` and the rest are in a vocabulary.

## Time

An exact count of nanoseconds, positive or negative. `ns us ms s min h d`; a `d` is 24 h; months are calendar, in a vocabulary.

| Signature | Fails | Means |
|---|---|---|
| `div(b: Time, mode: Rounding) -> Int` | `DivByZero` | whole `b`s in this time |
| `abs() -> Time` | - | `min max clamp` as for `Int` |

`Time + Time`, `Int * Time` are `Time`. `Time / Time` is `Float`; use `div` where the answer must be exact.

## Text, Bytes

| Signature | Fails | Means |
|---|---|---|
| `size: Int` | - | code points; on `Bytes`, bytes |
| `bytes: Bytes` | - | UTF-8 |
| `text() -> Text` | `NotText` | **new**, on `Bytes`; strict UTF-8 |
| `split(sep: Text) -> List[Text]` | - | keeps empty pieces; `"".split(",") == [""]`; `sep` not empty |
| `join(sep: Text) -> Text` | - | on `List[Text]` |
| `trim() -> Text` | - | both ends, Unicode white space |
| `lower() -> Text` | - | full mapping, no locale |
| `starts_with(x: Text) -> Bool` | - | |
| `contains(x: Text) -> Bool` | - | substring; `x in t` |
| `replace(old: Text, new: Text) -> Text` | - | every match, left to right; `old` not empty |
| `take(n: Int) -> Text` | - | first code points; `drop` likewise |

Unicode tables are pinned in the lock file. Escapes: `\n \t \\ \" \{ \u{1F600}`. `"{x}"` prints any data except `@secret`. `t[i]` is an error (`take`, `drop`); `b[i]` is a `u8`.

## List

| Signature | Fails | Means |
|---|---|---|
| `size: Int` | - | |
| `first: T` | `Missing` | `last` likewise |
| `get(i: Int) -> Option[T]` | - | `xs[i]` fails `Missing` |
| `filter(p: T -> Bool) -> List[T]` | - | keeps order |
| `map[U](f: T -> U) -> List[U]` | as `f` | earliest failure wins |
| `flat_map[U](f: T -> List[U]) -> List[U]` | as `f` | |
| `fold[A](start: A, f: (A, T) -> A) -> A` | as `f` | left to right |
| `sum[N](f: T -> N = it) -> N` | - | empty is zero; `Float` uses the fixed tree |
| `min[K](f: T -> K = it) -> T` | `Missing` | the element, not the key; `max` likewise |
| `all(p: T -> Bool) -> Bool` | - | empty is true; `any` of empty is false |
| `find(p: T -> Bool) -> Option[T]` | - | earliest |
| `position(p: T -> Bool) -> Option[Int]` | - | earliest |
| `contains(x: T) -> Bool` | - | `x in xs` |
| `distinct() -> List[T]` | - | keeps the earliest of equals |
| `sort[K](key: T -> K = it, desc: Bool = false) -> List[T]` | - | stable; `desc` **new** |
| `reversed() -> List[T]` | - | |
| `take(n: Int) -> List[T]` | - | `drop` likewise; `n >= 0` |
| `zip[U](ys: List[U]) -> List[(T, U)]` | `Uneven` | |
| `group_by[K](f: T -> K) -> Map[K, List[T]]` | - | keeps order inside a group |
| `push(x: T)` | - | adds at the end |
| `insert(at: Int, x: T)` | `Missing` | `at` in `0..=size` |
| `remove(at: Int)` | `Missing` | `xs[i] = x` replaces |

`sum` has `N` in `Int`, `Decimal`, `Money[C]`, `Float`, `Time`.

## Set, Map

| Signature | Fails | Means |
|---|---|---|
| `size: Int` | - | both |
| `list: List[T]` | - | **new**, on `Set`, ascending |
| `keys: List[K]` | - | ascending; `values` in key order |
| `entries: List[Entry[K, V]]` | - | `it.key`, `it.value` |
| `get(k: K) -> Option[V]` | - | `m[k]` fails `Missing` |
| `insert(x: T)` | - | on `Set`; already there: no change |
| `remove(k: K)` | - | both; absent: no change |

`Set` also has `filter map all any find sum min max` as `List`; `map` gives a `Set`. `m[k] = v` adds or replaces, so `Map` has no `insert`. `m[k].f = v` on a missing key fails `Missing`. `k in m` tests keys.

## Ranges

| Signature | Fails | Means |
|---|---|---|
| `size: Int` | - | `Int` ranges; zero when reversed |
| `list: List[Int]` | - | **new**; `Int` ranges |

`Int` ranges also have `map filter all any sum fold` and `for`. Any ordered type has `x in a..=b`. `x ± e` is `x - e ..= x + e`; a percent `e` beside a value that is not a percent is relative.

## Patterns

| Pattern | Matches |
|---|---|
| `3` `"a"` `none` `1..=5` | that value, that range |
| `_` | anything |
| `n` | in scope: equal to it; otherwise binds it |
| `Held(by, _)` `Held(until: u)` | variant or record, positional or by field |
| `(a, b)` | a tuple |

The first matching arm wins. Arms must cover every value; an arm that can never match is an error. A name bound by `is` on the left of `and` is in scope on the right, and after a `@guard`.

## Trace

| Signature | Fails | Means |
|---|---|---|
| `steps: List[Step]` | - | `event reply at before after`; refused calls too, with `before == after` |
| `states: List[World]` | - | the start, then after each step |
| `eventually(p: World -> Bool) -> Bool` | - | some state from here on |
| `within(d: Time, p: World -> Bool) -> Bool` | - | the same, no later than `d` |

`reply` is a `Result`. "Here" is the step or state being visited when the call is inside `t.steps` or `t.states`, otherwise the start. `world.name` is a `state`; an `Atomic` field reads as its plain value.

## Id

| Signature | Fails | Means |
|---|---|---|
| `fresh() -> Id[T]` | - | on `Id`; effect `random.read`; never repeats |
| `text: Text` | - | **new**; stable printed form |

`Id(1)` builds one in tests. The order of fresh ids is not part of the meaning: the interpreter permutes it by seed, so a test that leans on it fails under another seed.

## World handlers

Usable as calls in a test, or in `@given`. Rows marked **new** are proposals ([06](06-logic-and-tests.md)).

| Signature | Effect | Means |
|---|---|---|
| `now() -> Time` | `clock.read` | since the world began; the same for a whole rule |
| `clock.at(t: Time)` | test | sets the time; fires nothing |
| `clock.advance(d: Time)` | test | fires all due timers and deadlines in time order, then source order |
| `http.call(url: Url, body: Data) -> Reply` | `http.call` | task only; fails `Unreachable`; a 503 is a `Reply`. **new**: further named arguments (`topic:`, `attempt:`) travel with the request and read back as fields of the `Call` |
| `http.reply(status: Int, body: Data = none)` | test | the answer from now on |
| `http.calls: List[Call]` | test | `url body key at`, in order, whether or not the reply arrived |
| `net.drop(n: Int, leg: Leg = reply)` | test | loses the next `n`; `request` **new** |
| `net.duplicate(n: Int)` | test | **new**; delivers the next `n` twice |
| `random.int(r: Range) -> Int` | `random.read` | |
| `random.token() -> Token` | `random.read` | unguessable in software, seeded in tests |
| `random.seed(n: Int)` | test | **new** |
| `store.conflict(n: Int)` | test | **new**; the next `n` commits lose a race and rerun |
| `crash()` | test | **new**; stop and restart here; in a `Trace`, anywhere |
| `tasks.drain()` | test | **new**; runs started tasks until none is due |

`@given(place -> value)` replaces that place. Handlers apply left to right.

## Machine altitude

| Signature | Fails | Means |
|---|---|---|
| `load(o: Ordering) -> T` | - | on `Atomic[T]`; `relaxed acquire release seq` ([23](23-memory-concurrency-distribution.md)) |
| `store(x: T, o: Ordering)` | - | |
| `replace_if(old: T, new: T, o: Ordering)` | `Changed` | **new**; one atomic step |
| `read() -> T` | - | on an `@mmio` field; effect `field.read` |
| `write(x: T)` | - | on an `@mmio` field; effect `field.write` |
| `size: Int` | - | on `Array[T, n]`, known at build |

`T` in `Atomic[T]` is a sized integer or `Bool`. Atomic read-modify-write is the one exception to the rule that a name never both changes and returns: the old value is the operation, so `fetch_add` returns it. Loop around `replace_if`; a backend may emit the single instruction. `Array(fill: 0)` takes `n` from the type; an index is an obligation, never a hidden check.

## The trial's questions

Three authors with only a card left 160, 183 and 35 lines containing `GUESS` (the third had a later, fuller card). The library ones, answered:

| Guess | Answer |
|---|---|
| `len`, `add`, `sort_by`, `skip`, `desc` | `size`, `insert`, `sort`, `drop`, `desc: true` |
| a string type, a duration type | `Text`, `Time` |
| a missing key reads as false or the first variant | `Missing`; give a default with `else` |
| `push` returns the new list | it returns nothing; `row.push(0);` |
| `last` is an element or an `Option` | the element, or `Missing` |
| `fold` arguments | start, then `(acc, x)` |
| `Int ÷ Int`, `Time ÷ Time` | `div` with a mode; `Float` |
| how to make an id, a token | `Id.fresh()`, `random.token()` |
| two loops may reuse `i` | yes: the first `i` is out of scope |
| does `advance` fire `every`? | yes |
| atomics in a trace state | plain values |
| catching a failure | `try_call` or `else` |

Roles, visibility, mail and signing: [27](27-authorization.md), [29](29-first-vocabularies.md).

## Limits

- **Pinned tables age.** A store or client with other Unicode or currency tables disagrees until regenerated.
- **Sorted iteration costs.** A hash map must sort to be observed. A backend may skip it only where it shows order cannot be seen.
- **Code points are not what users count.** A family emoji is several; a UI needs a grapheme vocabulary.
- **`Time` is nanoseconds.** Real clocks are coarser; a backend states its grain.
- **`eventually` depends on where it is written.** If authors misread "here", it becomes an explicit `s.eventually(p)`.
- Only the signatures were checked, and only that they parse. Whether the names suffice is unknown; the next trial counts what authors reach for and do not find.
