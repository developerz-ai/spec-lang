# Domain tour

> **Design.** Nothing here runs yet. One short program per domain, to show what one declaration shape covers and where it stops.

Every program is rules over state, plus tasks where it must wait on the outside ([card](01-language-card.md)). A domain adds kinds, notes and effects through `use`; it never adds grammar. Each program below parses. Kinds, notes and library names that are not on the card (`bucket`, `plant`, `@sync`, `lidar`) are **proposals** for that vocabulary; their rules live in [09](09-vocabularies.md).

Three questions per domain: what is **derived** (never written), what stays **foreign** (reached by contract, evidence `assumed`), and what is **hard**. No backend exists yet.

Eight longer programs are in [24](24-examples.md): [seats](24-examples.md#seats-pure-logic-scenarios-laws), [todo](24-examples.md#todo-a-web-feature), [movement](24-examples.md#game-movement), [k8s service](24-examples.md#kubernetes-service), [PID](24-examples.md#pid-controller), [page allocator](24-examples.md#kernel-page-allocator), [B-tree + WAL](24-examples.md#b-tree-page-and-wal), [webhooks](24-examples.md#webhooks-tasks-retries-signing). Games are covered in [13](13-games.md), kernels and databases in [14](14-systems.md).

## Web: pay for an order

```slg
use web;
use payments;

type Status = Open | Paying | Paid | Refused;

@access(owner -> read)
entity Order(owner: Id[User], total: Money[USD] where self > 0USD, status: Status = Open);

event paying(order: Id[Order]);

@edge(Open -> Paying, via: pay, by: owner)
@edge(Paying -> Paid, via: settle)
@edge(Paying -> Refused, via: settle)
flow OrderLife(at: Order.status);

@on(http.post("/orders/pay"))
@access(owner -> call)
@uses(Order.write, paying.emit)
@guard(Order[id].status == Open else Closed)
pub rule pay(by: Id[User], id: Id[Order]) = {
  Order[id].status = Paying;
  paying.emit(id);
}

@guard(Order[id].status == Paying else Closed)
rule settle(id: Id[Order], ok: Bool) = { Order[id].status = if ok then Paid else Refused; }

@on(paying)
@retry(5, backoff: 2s, on: Unreachable)
@idempotent(order)
task charge(order: Id[Order]) = {
  let o = Order[order];
  let reply = payments.charge(o.owner, amount: o.total, key: order) else fail Unreachable;
  settle(order, ok: reply.accepted);
}
```

| | |
|---|---|
| Derived | table, migration, route, client, outbox, resumable task, contract tests |
| Foreign | the payment provider |
| Hard | exactly once is `assumed`: it rests on the provider honouring `key`. After five failures the order stays `Paying`; the program above does not say what then, and a liveness law would refute it |

## Mobile: record a run offline

```slg
use mobile;
use crdt;

type Point(at: Instant, lat: Float, lon: Float);

@access(owner -> all)
@sync(on: online, merge: merge)
entity Run(owner: Id[User], @pii points: Set[Point] = Set());

fn merge(a: Run, b: Run) -> Run = a.with(points: crdt.union(a.points, b.points));

@on(location.update(every: 5s))
@permission(location, why: "draw your route")
@access(owner -> call)
@uses(Run.write)
pub rule record(by: Id[User], id: Id[Run], p: Point) = { Run[id].points.insert(p); }

law merge_commutes(a: Run, b: Run) = merge(a, b).points == merge(b, a).points;
```

| | |
|---|---|
| Derived | local store, outbox, permission prompt text, background mode declaration, Swift and Kotlin models |
| Foreign | map views, sensors, push services |
| Hard | the operating system may stop background work; `every: 5s` is a request. Merges with deletes need more than a set union |

## Desktop: undo

```slg
use desktop;
use text;

type Edit(at: Int where self >= 0, old: Text, new: Text);
state page: Text = "";
state done: List[Edit] = [];

fn apply(t: Text, e: Edit) -> Text = text.splice(t, at: e.at, cut: e.old.size, add: e.new);
fn invert(e: Edit) -> Edit = Edit(e.at, old: e.new, new: e.old);

@guard(text.slice(page, at: e.at, size: e.old.size) == e.old else Conflict)
rule change(e: Edit) = { page = apply(page, e); done.push(e); }

@on(key("ctrl+z"))
@guard(done.size > 0 else NothingToUndo)
rule undo() = {
  let e = done.last else fail NothingToUndo;
  page = apply(page, invert(e));
  done = done.take(done.size - 1);
}

law undo_restores(t: Text, e: Edit) =
  text.slice(t, at: e.at, size: e.old.size) == e.old implies apply(apply(t, e), invert(e)) == t;
```

| | |
|---|---|
| Derived | menu entry, shortcut per platform, crash-safe document state |
| Foreign | native widgets, file dialogs, accessibility trees |
| Hard | platform feel. Text positions mean code points here; editors also need grapheme clusters |

## Devops: a database that proves its backups

```slg
use infra;

@target(terraform.aws)
@region(eu)
@encrypted
@backup(every: 1h, keep: 30d)
@access(Api -> (read, write))
database Orders(engine: Engine = postgres, disk: Bytes = 100GiB);

@on(every(7d))
@within(2h)
@uses(infra.restore, alert.send)
task restore_drill() = {
  let copy = Orders.restore(latest) else fail RestoreFailed;
  assert copy.age <= 1h;
  copy.destroy();
}

@given(cloud.sim(), Orders.backups -> [])
test drill_fails_without_backups = { restore_drill() -> fail RestoreFailed; }
```

| | |
|---|---|
| Derived | Terraform, network rules from `@access`, the schedule, a plan to review |
| Foreign | the cloud provider |
| Hard | a simulated cloud proves the logic, not the account. Drift and quotas appear only in the real one |

## Embedded: debounce a button

```slg
use embedded;

state last: Time = 0s;
state presses: 0..=255 = 0;

@target(cortex_m0)
@on(irq(5))
@alloc(none)
@within(10us)
rule button() = {
  let t = now();
  if t - last >= 20ms { presses = (presses + 1).min(255); }
  last = t;
}

@given(clock.at(1s))
test bounce_counts_once = {
  button();
  clock.advance(5ms);
  button();
  assert presses == 1;
}
```

| | |
|---|---|
| Derived | vector entry, critical section around shared state, integer width from the range |
| Foreign | vendor peripheral drivers |
| Hard | `@within(10us)` needs a timing model of the chip; without one it is `assumed` or measured on hardware |

## Robotics: slow down near obstacles

```slg
use robotics;

type Drive(speed: Float[m/s], turn: Float[rad/s]);

fn speed(clear: Float[m]) -> Float[m/s] =
  if clear < 0.5m then 0[m/s] else if clear < 2m then 0.3[m/s] else 1[m/s];

@target(ros2)
@on(every(20ms))
@within(5ms)
controller Follow(clear: Float[m], bearing: Float[rad]) -> Drive =
  Drive(speed(clear), turn: bearing.clamp(-1rad, 1rad) / 1s);

law closer_is_never_faster(a: 0m..=10m, b: 0m..=10m) = a <= b implies speed(a) <= speed(b);
```

| | |
|---|---|
| Derived | node, topics, timer, units check, replay of recorded runs |
| Foreign | perception, localisation, motor firmware |
| Hard | the law covers `speed`, not the room. Lidar error and braking distance need a plant model, and that model is `assumed` |

## Data and ML: promote a model only if it is better

```slg
use ml;

type Label = Good | Bad;

@split(by: user, train: 80%, test: 20%)
dataset Reviews(user: Id[User], @pii text: Text, label: Label);

@uses(gpu)
task train(d: Data[Reviews], seed: Int) -> Model[Text, Label] = foreign python "train.py:fit";

state live: Model[Text, Label];

@uses(live.write)
@guard(accuracy(m, on: Reviews.test) >= accuracy(live, on: Reviews.test) + 1% else NotBetter)
pub rule promote(m: Model[Text, Label]) = { live = m; }

law no_user_in_both =
  Reviews.train.all(a => Reviews.test.all(b => a.user != b.user));
```

| | |
|---|---|
| Derived | split, lineage of data and seed, promotion gate, label flow of `@pii` |
| Foreign | the training loop and its kernels |
| Hard | accuracy is `sampled`, never `proven`; repeated promotions against one test set overfit it |

## Scientific: a pendulum

```slg
use science;

const G: Float[m/s²] = 9.81[m/s²];

plant Pendulum(angle: Float[rad], rate: Float[rad/s], length: 0.5m..=2m) = {
  der(angle) = rate;
  der(rate) = -(G / length) * sin(angle) * 1rad;
}

fn energy(p: Pendulum) -> Float[J/kg] =
  (p.rate * p.length / 1rad)^2 / 2 + G * p.length * (1 - cos(p.angle));

law energy_holds(a: 0.1rad..=1rad, l: 0.5m..=2m) =
  solve(Pendulum(a, rate: 0[rad/s], length: l), span: 10s, step: 1ms, method: verlet)
    .all(energy(it) in energy(Pendulum(a, rate: 0[rad/s], length: l)) ± 0.1%);
```

| | |
|---|---|
| Derived | solver loop, unit check, vectorised backend, parameter sweep |
| Foreign | BLAS, stiff solvers, meshers |
| Hard | floating point differs across backends; the claim is a tolerance, `sampled`. The 0.1% has not been run |

## Distributed: a lease with fencing

```slg
use cluster;

type Lease(holder: Id[Node], token: Int = 0, until: Time = 0s);
state lease: Lease = Lease(Id(0));
state accepted: Int = 0;
state value: Text = "";
const a: Id[Node] = Id(1);
const b: Id[Node] = Id(2);

@guard(now() >= lease.until else Held)
rule acquire(node: Id[Node]) -> Int = {
  lease = Lease(node, token: lease.token + 1, until: now() + 10s);
  return lease.token;
}

@guard(token >= accepted else Stale)
rule write(token: Int, v: Text) = { accepted = token; value = v; }

law never_goes_back(t: Trace[acquire | write | crash]) =
  t.steps.all(it.after.accepted >= it.before.accepted);

@given(clock.at(0s))
test late_holder_is_refused = {
  acquire(a) -> 1;
  clock.advance(10s);
  acquire(b) -> 2;
  write(2, v: "new");
  write(1, v: "old") -> fail Stale;
}
```

| | |
|---|---|
| Derived | RPC types, message encoding, partition and crash schedules for traces |
| Foreign | the consensus store that makes `lease` one place |
| Hard | exploration is `bounded(k)`. Clock skew breaks the lease, not the fence: `accepted` is what keeps it safe |

## Compiler pass: fold constants

```slg
type Expr = Lit(n: Int) | Var(name: Text) | Add(a: Expr, b: Expr) | Mul(a: Expr, b: Expr);

fn eval(e: Expr, env: Map[Text, Int]) -> Int = match e {
  Lit(n) => n;
  Var(name) => env[name] else 0;
  Add(a, b) => eval(a, env) + eval(b, env);
  Mul(a, b) => eval(a, env) * eval(b, env);
}

fn simplify(e: Expr) -> Expr = match e {
  Add(a, b) => plus(simplify(a), b: simplify(b));
  Mul(a, b) => times(simplify(a), b: simplify(b));
  _ => e;
}

fn plus(a: Expr, b: Expr) -> Expr = match (a, b) {
  (Lit(x), Lit(y)) => Lit(x + y);
  (Lit(0), _) => b;
  (_, Lit(0)) => a;
  _ => Add(a, b: b);
}

fn times(a: Expr, b: Expr) -> Expr = match (a, b) {
  (Lit(x), Lit(y)) => Lit(x * y);
  (Lit(1), _) => b;
  (_, Lit(1)) => a;
  _ => Mul(a, b: b);
}

law meaning_kept(e: Expr, env: Map[Text, Int]) = eval(simplify(e), env) == eval(e, env);
```

| | |
|---|---|
| Derived | generators for `Expr`, shrinking to the smallest wrong program |
| Foreign | object formats, linkers |
| Hard | proving `meaning_kept` needs induction over `Expr`; until a prover does it, it is `sampled` |

## CLI: count errors in a log

```slg
use cli;

@on(cli("top-errors"))
@uses(fs.read, out.write)
pub task top(file: Path, n: 1..=100 = 10) = {
  let errors = fs.read(file).split("\n").filter(it.starts_with("ERROR "));
  for e in errors.group_by(it).entries.sort((-it.value.size, it.key)).take(n) {
    out.line("{e.value.size} {e.key}");
  }
}

@given(fs.file("a.log" -> "ERROR x\nok\nERROR y\nERROR x"))
test counts_and_sorts = {
  top("a.log");
  assert out.text == "2 ERROR x\n1 ERROR y\n";
}
```

| | |
|---|---|
| Derived | flag parsing, `--help`, `--json`, exit code per error, a dry run from handlers |
| Foreign | other programs it calls |
| Hard | startup time of the chosen backend; streaming files larger than memory is an open choice (`E-OPEN`) |

## Limits

- The language's designer wrote these. They show reach; whether models write it well is the trial in [02](02-syntax.md).
- Parsing is the only check that ran. Types, units and laws were reviewed by eye.
- The short ones flatter the language: most of a real mobile or desktop app is platform behaviour that stays foreign. [12](12-frameworks.md) and [19](19-density-and-experiments.md) say how the share is to be measured.
- Each vocabulary needs its own card, and a model pays for reading it. Eleven domains mean eleven taxes.
