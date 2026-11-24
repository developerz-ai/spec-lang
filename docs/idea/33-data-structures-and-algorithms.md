# Data structures and algorithms

> **Design.** Nothing here runs yet. How to write algorithms, who picks the data structure, how cost is claimed, and what is taken from Haskell. The programs parse; the logic of binary search, merge sort, tree insert, shortest paths, union-find and three-way merge was checked as Python transliterations against naive versions on 4,000 random inputs each.

**Say what the collection means; let the compiler pick how it is stored; write the algorithm when it is yours.** Databases have done this for decades: a query says which rows, an index decides how.

## Three ways down

| You write | The compiler | Use when |
|---|---|---|
| operations: `xs.sort(it.due)`, `m[k]`, `xs.min()` | picks the representation from how it is used, and pins it in the lock file | nearly always |
| an algorithm in steps | compiles it as written | the algorithm is the logic |
| `machine { }` layouts | keeps every byte | allocators, page formats, drivers |

| Seen in the program | A backend may choose |
|---|---|
| `min()` then `remove` in a loop | a heap |
| `push` at one end, `remove(0)` at the other | a ring buffer |
| `x in xs` in a loop | a hash set beside the list |
| `filter(it.owner == by).sort(it.due)` on stored data | an index on `(owner, due)` |
| a record list read one field at a time | columns instead of rows |

The result must be identical, order included ([28](28-standard-library.md)). Only cost may change. Override with a pin: `@store(heap)` on the `var` or `state`.

## Recursive types

```slg
type Tree[T] = Leaf | Node(left: Tree[T], value: T, right: Tree[T]);

fn insert(t: Tree[Int], x: Int) -> Tree[Int] = match t {
  Leaf => Node(Leaf, value: x, right: Leaf);
  Node(l, v, r) =>
    if x < v then Node(insert(l, x: x), value: v, right: r)
    else if x > v then Node(l, value: v, right: insert(r, x: x))
    else t;
}

fn items(t: Tree[Int]) -> List[Int] = match t {
  Leaf => [];
  Node(l, v, r) => items(l) + [v] + items(r);
}

law sorted_and_unique(xs: List[Int]) =
  items(xs.fold(Leaf, (t, x) => insert(t, x: x))) == xs.distinct().sort();
```

Values never change, so `insert` shares the parts it does not touch. Recursion must shrink an argument, or carry a measure the checker can follow; otherwise termination is a claim like any other.

## Search, with a cost claim

```slg
@pre(xs == xs.sort())
@steps(xs.size.log2() + 1)
fn lower(xs: List[Int], x: Int) -> Int = {
  var lo = 0;
  var hi = xs.size;
  while lo < hi {
    let mid = lo + (hi - lo).div(2, floor);
    if xs[mid] < x { lo = mid + 1; } else { hi = mid; }
  }
  return lo;
}

law splits(xs: List[Int], x: Int) =
  lower(xs.sort(), x: x) == xs.filter(it < x).size;
```

`@steps(f)` is a claim about work: loop turns and calls, counted by the interpreter. The count is exact and the same on every machine, so cost can be **tested** like any other claim, with a counterexample: the input that took too many steps. A wrong `O(n)` fails on the first large generated case. Time on real hardware stays a separate, measured claim (`@within`).

## Merging

```slg
type Item(key: Int, tag: Int);

@pre(a == a.sort(it.key) and b == b.sort(it.key))
fn merge(a: List[Item], b: List[Item]) -> List[Item] = {
  var out: List[Item] = [];
  var i = 0;
  var j = 0;
  while i < a.size or j < b.size {
    if j == b.size or (i < a.size and a[i].key <= b[j].key) {
      out.push(a[i]);
      i = i + 1;
    } else {
      out.push(b[j]);
      j = j + 1;
    }
  }
  return out;
}

fn sorted(xs: List[Item]) -> List[Item] =
  if xs.size <= 1 then xs
  else merge(sorted(xs.take(xs.size.div(2, floor))), b: sorted(xs.drop(xs.size.div(2, floor))));

law stable(xs: List[Item]) = sorted(xs) == xs.sort(it.key);

type Merged = Same(value: Int) | Conflict(left: Int, right: Int);

fn three_way(base: Int, left: Int, right: Int) -> Merged =
  if left == right then Same(left)
  else if left == base then Same(right)
  else if right == base then Same(left)
  else Conflict(left, right: right);

law no_silent_loss(b: Int, l: Int, r: Int) =
  (l != b and r != b and l != r) implies three_way(b, left: l, right: r).is(Conflict(_, _));
```

Merging copies that changed apart is the same idea with a law instead of a conflict: a `Replicated` value joins with a function that must be commutative, associative and idempotent, and those three are checked ([23](23-memory-concurrency-distribution.md)).

## Graphs

```slg
type Edge(to: Int, cost: Int where self >= 0);
type Stop(far: Int, at: Int);

fn dist(g: Map[Int, List[Edge]], start: Int) -> Map[Int, Int] = {
  var best: Map[Int, Int] = Map();
  var todo = [Stop(0, at: start)];
  while todo.size > 0 {
    let n = todo.min(it.far);
    todo.remove(todo.position(it == n) else 0);
    if not (n.at in best) {
      best.insert(n.at, n.far);
      for e in g.get(n.at) else [] {
        if not (e.to in best) { todo.push(Stop(n.far + e.cost, at: e.to)); }
      }
    }
  }
  return best;
}

law no_shortcut(g: Map[Int, List[Edge]], s: Int) =
  dist(g, start: s).entries().all((u, d) =>
    (g.get(u) else []).all(e => dist(g, start: s)[e.to] <= d + e.cost));
```

The reading is the textbook one. `min` then `remove` is what lets a backend use a heap.

## Monads, Haskell, and what we take

A monad is one shape for "do this, then that, with something extra carried along". Haskell makes the shape a programmable type class. Here each "something extra" that matters has a fixed form, so there is one way to write it and nothing to stack.

| Haskell | Carries | Here |
|---|---|---|
| `Maybe` | might be absent | `Option`, `else`, `try` |
| `Either e` | might fail | `fail`, `else`, `try`, `@fails` |
| `List` | many results | `flat_map`, `filter` |
| `State` | changing state | `rule`, or `var` inside a `fn` |
| `Reader` | an environment | handlers named by `app` |
| `Writer` | a log | an `event` or a `state` list |
| `IO` | the world | effects in `@uses` |
| `STM` | atomic update | a `rule` |
| `Async` | waiting | a `task`, `all`, `race` |
| `Cont`, free monads | reified control | a `task`'s steps; handlers interpret effects |
| `do` notation | sequence | a block |
| transformer stacks | several at once | effects and failures are sets, in no order |

| Taken from Haskell and its family | Form |
|---|---|
| purity by default | `fn` writes nothing |
| algebraic types and exhaustive `match` | `type A = B \| C(x: T)`; a missing case is an error |
| properties (QuickCheck) | a `test` or `law` with parameters |
| holes that report type and scope | `?name` |
| equational reasoning | a law may be used by the optimiser as a rewrite, because it is checked |
| total functions | termination is a claim |

| Left out | Why |
|---|---|
| user-defined monads, higher-kinded types | many ways to write one thing; errors far from the cause; models mix the dialects |
| type classes with instances anywhere | the meaning of `x == y` would depend on an import. Equality, order and hashing are structural |
| laziness by default | cost and effects become hard to see; ranges and queries are lazy where it cannot be observed |
| point-free style, custom operators | hard to edit locally |

Algebraic structures are ordinary values with laws, passed by name:

```slg
type Monoid[T](empty: T, join: (T, T) -> T);

fn reduce[T](xs: List[T], m: Monoid[T]) -> T = xs.fold(m.empty, m.join);

const add: Monoid[Int] = Monoid(0, join: (a, b) => a + b);

law joins_either_way(a: Int, b: Int, c: Int) =
  add.join(a, add.join(b, c)) == add.join(add.join(a, b), c);

law empty_changes_nothing(a: Int) =
  add.join(add.empty, a) == a and add.join(a, add.empty) == a;
```

The second law matters: `max` with an empty of 0 looks right and fails it at the first negative number. A lawful `join` is what lets a backend split the work across cores; without the law it runs left to right.

## Elixir and the rest of the family

| Language | Taken | Form here | Left out |
|---|---|---|---|
| Elixir | pipelines read in order of work | `xs.filter(...).sort(...)`: one call syntax | a second spelling for the same call |
| | function heads that match | `match`, with every case covered | several heads spread over a file |
| | `with`: a chain of steps that may fail | `else fail Err` on each step; `try` | - |
| | changesets: valid or a list of reasons | types with claims, guards with named errors | a separate validation object |
| | doctests | `@cases`, `test` beside the logic | - |
| Erlang | isolated state, messages, let it fail | module-owned state, events, tasks that resume | untyped mailboxes, hot code loading |
| | supervision trees | `@retry`, `exhausted:`, a `flow` deadline | restart as the cure for a logic error |
| OCaml, F# | sums with exhaustive `match`; modules with signatures | the same; `pub` is the signature | functors |
| F# | units of measure | units on every number | - |
| Scala | one language for objects and functions | - | implicits, many styles |
| Elm | no runtime exceptions; model, message, update | `rule`: state + event -> new state + effects | one fixed architecture |
| Clojure | values apart from places; generative tests | `let` and `state`; property tests | macros, no static types |
| Koka | effects in the type, inferred | `@uses` | user-defined handlers as control flow |
| Gleam, ReScript | a small typed language on an existing runtime | a framework as backend | - |
| Idris, Agda, Lean | types that state theorems; holes | claims, `?holes`, laws | proof terms as daily work |

The common thread is functional core, effects at the edge. spec-lang makes that the only shape: a `fn` is the core, a `rule` is the edge of the state, a `task` is the edge of the world.

## Catalogue

| Need | Write | Stays with the author |
|---|---|---|
| list, set, map, sort, search, group | core library | - |
| queue, stack, deque, priority queue | `List` operations; a pin if the choice matters | - |
| LRU or size-bounded cache | `@memo(size:)`, or a `Map` with a last-used field | eviction when it is a business rule |
| tree, trie, rope, interval tree | recursive type | shape and balance |
| graph | `Map[Node, List[Edge]]` | traversal |
| union-find | a `Map` or `Array` of parents | yes |
| spatial index, full-text, vectors | a vocabulary | tuning |
| dynamic programming | a recursive `fn` with `@memo` | the recurrence |
| matrices, tensors | a vocabulary over `Array` | kernels as a `@fast` pair ([31](31-dropping-down.md)) |
| crypto, compression | `foreign`, tested against published vectors | never hand-written |
| lock-free structures | `machine`, `Atomic` | all of it, and little evidence |

## Limits

- Plain algorithms are already mostly logic, so the text is about as long as Python's; both blind authors said so. The gain is laws, cost claims and one version for every backend.
- Choosing a data structure from use is a compiler problem with a long literature and few general successes. A pin will often be needed; a wrong choice must at least be visible in the plan.
- The interpreter's step count ignores caches, allocation and vector units.
- `no_shortcut` calls `dist` for every pair, which is slow as a test, and it shows no path is too long without showing every reachable node is found.
- Leaving out type classes means generic numeric code takes its operations as values, which is longer than in Haskell.
- `@store`, `@steps` and `log2` are proposed here; neither the card nor [28](28-standard-library.md) lists them.
- The family tables are background knowledge, not sourced line by line; [17](17-inspirations.md) holds the ones that were looked up.
