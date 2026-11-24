# Examples

> **Design.** Nothing here runs yet. Whole programs that parse with the [grammar](25-grammar.md); types, names and meaning are unchecked.

## Seats: pure logic, scenarios, laws

```slg
type Seat = Free | Held(by: Id[User], until: Time) | Sold(to: Id[User]);
type SeatNo = 1..=200;
state seats: Map[SeatNo, Seat];
const ana: Id[User] = Id(1);
const bob: Id[User] = Id(2);

@pre(base >= 0USD)
@post(0USD <= result <= base)
@cases((100USD, false, 3d) -> 100USD, (100USD, false, 30d) -> 90USD, (100USD, true, 30d) -> 72USD)
fn price(base: Money[USD], vip: Bool, early: Time) -> Money[USD] =
  (base * (if vip then 80% else 100%) * (if early >= 14d then 90% else 100%)).round(half_even);

fn open(s: Seat, t: Time) -> Bool = match s {
  Free => true;
  Held(_, until) => t >= until;
  Sold(_) => false;
}

@why("a buyer gets ten minutes to pay")
@uses(seats.write, clock.read)
@guard(open(seats[n], now()) else Taken)
pub rule hold(by: Id[User], n: SeatNo) -> Time = {
  let until = now() + 10min;
  seats[n] = Held(by, until);
  return until;
}

@uses(seats.write, clock.read)
@guard(seats[n].is(Held(by, until)) and now() < until else NotHeld)
pub rule buy(by: Id[User], n: SeatNo) = { seats[n] = Sold(by); }

rule release(n: SeatNo) = ?release;

law sold_is_final(t: Trace[hold | buy | release], n: SeatNo) =
  t.steps.all(it.before[n].is(Sold(_)) implies it.after[n] == it.before[n]);

law vip_never_pays_more(b: Money[USD] where self >= 0USD, e: Time) =
  price(b, true, e) <= price(b, false, e);

@given(clock.at(0s), seats[1] -> Free)
test ana_holds = {
  hold(ana, 1) -> 10min;
  hold(bob, 1) -> fail Taken;
}

@why("a seat nobody paid for goes back on sale")
@given(ana_holds)
test unpaid_hold_lapses = {
  clock.advance(10min);
  buy(ana, 1) -> fail NotHeld;
  hold(bob, 1) -> 20min;
  seats[1] -> Held(bob, 20min);
}
```

## Todo: a web feature

```slg
use web;

@why("a user tracks only their own work")
@tenant(owner)
@index(owner, done, due)
@access(owner -> all)
entity Task(
  id: Id[Task],
  owner: Id[User],
  @pii title: Text[1..=200],
  due: Option[Date] = none,
  done: Bool = false,
);

@uses(Task.write)
@access(Task[id].owner == by)
@post(result.done)
pub rule complete(by: Id[User], id: Id[Task]) -> Task = {
  Task[id].done = true;
  return Task[id];
}

@uses(Task.read)
@live
@page(20)
pub query mine(by: Id[User]) -> List[Task] =
  Task.filter(it.owner == by and not it.done).sort(it.due);

@at("/")
@access(by in User)
page Dashboard(by: Id[User], tasks: Live[List[Task]] = mine(by)) = column(
  heading("Today"),
  form(create, fields: (title, due)),
  list(tasks, empty: text("Nothing due"), row: check(it.title, on: it.done, toggle: complete(by, it.id))),
);

const ana: Id[User] = Id(1);

test no_such_task = { complete(ana, id: Id(9)) -> fail NotFound; }

@target(ultimate)
app Todo(rules: [complete], pages: [Dashboard]);
```

## Webhooks: tasks, retries, signing

```slg
use web;
use errors;
use jobs;
use postly.orgs;
use postly.posts;

const CAP: Int = 10;
const DISABLE_AFTER: Int = 10;
const TOPIC: Text = "post.published";
const HOOK: Url = url"https://hooks.example.com/postly/1";

@why("where an org hears about its posts: a handful of receivers, never a broadcast list")
@tenant(org)
@index(org, created)
entity Endpoint(
  id: Id[Endpoint],
  org: Id[orgs.Org],
  url: Url,
  @secret secret: Hex[32],
  disabled: Option[Text[1..=200]] = none,
  created: Instant,
);

@why("every attempt is a fact: appended, never updated")
@tenant(org)
@append
@unique(endpoint, seq)
@index(org, endpoint, seq)
entity Delivery(
  id: Id[Delivery],
  org: Id[orgs.Org],
  @cascade endpoint: Id[Endpoint],
  seq: Int where self >= 1,
  webhook: Text[1..=80],
  event: Text[1..=128],
  topic: Text[1..=80],
  attempt: Int,
  ok: Bool,
  status: Option[Int] = none,
  took: Time,
  error: Option[Text[1..=500]] = none,
  streak: Int,
  at: Instant,
);

type Issued(id: Id[Endpoint], url: Url, secret: Hex[32]);
type Announced(id: Id[posts.Post], slug: Text, title: Text, at: Option[Instant]);

@status(409)
@fix("remove(org: {org}, id) as the org's owner frees a slot")
error LimitReached(org: Id[orgs.Org], limit: Int) =
  "org {org} already has {limit} webhook endpoints, the most one org may register";

@status(422)
@fix("add(org, url: 'https://hooks.example.com/postly'): a public https receiver, http only locally")
error UrlRefused(reason: Text) = "the webhook receiver URL {reason}";

@status(404)
@fix("list the org's endpoints and pass one of their ids")
error NotFound(id: Id[Endpoint]) = "no webhook endpoint {id} in the acting member's org";

fn administers(by: Id[orgs.Member], org: Id[orgs.Org]) -> Bool = foreign ts "@postly/core#mayAdministerOrg";

@why("a URL the delivery screen refuses would dead-letter every publication and never be disabled")
@fails(UrlRefused)
fn screen(url: Url, local: Bool) -> Url = {
  let kind = net.kind(url.host);
  if url.scheme != "https" and not (url.scheme == "http" and local) {
    fail UrlRefused("is {url.scheme}: a signed delivery goes over https");
  }
  if url.host.split(".").last == "localhost" { fail UrlRefused("names this machine"); }
  if kind != Public and kind != Name {
    fail UrlRefused("is a {kind} address, a delivery only opens a public one");
  }
  return url;
}

@why("the secret is answered here once and never readable again, so no agent surface")
@uses(Endpoint.write, random.read, clock.read, env.read)
@access(administers(by, org) else Forbidden)
@guard(Endpoint.filter(it.org == org).size < CAP else LimitReached(org, CAP))
@fails(UrlRefused)
@reveals(secret)
pub rule add(by: Id[orgs.Member], org: Id[orgs.Org], url: Url) -> Issued = {
  let ok = screen(url, local: env.local);
  let id = Id.fresh();
  let secret = random.token();
  Endpoint.insert(Endpoint(id, org: org, url: ok, secret: secret, created: clock.instant()));
  return Issued(id, url: ok, secret: secret);
}

@why("revoking is the only remedy for a leaked secret")
@uses(Endpoint.write)
@access(administers(by, org) else Forbidden)
@guard(id in Endpoint and Endpoint[id].org == org else NotFound(id))
pub rule remove(by: Id[orgs.Member], org: Id[orgs.Org], id: Id[Endpoint]) -> Id[Endpoint] = {
  Endpoint.remove(id);
  return id;
}

query live(org: Id[orgs.Org]) -> List[Id[Endpoint]] =
  Endpoint.filter(it.org == org and it.disabled == none).sort(it.created).map(it.id);

@why("announced in the publisher's own commit: a rolled-back publish tells nobody")
@on(posts.published)
@uses(Endpoint.read, posts.Post.read)
rule announce(post: Id[posts.Post]) = {
  for e in live(posts.Post[post].org) { deliver.start(post, endpoint: e); }
}

type Outcome = Answered(status: Int) | Failed(error: Text);

fn delivered(o: Outcome) -> Bool = match o {
  Answered(status) => status < 300;
  Failed(_) => false;
}

@why("retry and disabling are facts about one endpoint, so one delivery is one task")
@retry(8, backoff: exponential, on: Unreachable)
@within(30s)
@idempotent((post, endpoint))
@uses(http.call, clock.read, Endpoint.read, posts.Post.read)
@fails(Unreachable)
task deliver(post: Id[posts.Post], endpoint: Id[Endpoint]) = {
  if not (endpoint in Endpoint) { return; }
  let e = Endpoint[endpoint];
  let p = posts.Post[post];
  if e.disabled != none or p.status != posts.Published { return; }
  let body = Announced(p.id, slug: p.slug, title: p.title, at: p.published_at);
  let began = clock.instant();
  let out = match try_call(http.call(e.url, topic: TOPIC, sign: e.secret, body: body)) {
    Ok(reply) => Answered(reply.status);
    Err(x) => Failed(x.text);
  }
  record(e.id, post: post, attempt: run.attempt, out: out, took: clock.instant() - began);
  if not delivered(out) { fail Unreachable; }
}

@why("the streak is stored on the row, so the next attempt reads one row, not the history")
@idempotent((endpoint, post, attempt))
@uses(Delivery.write, Endpoint.write, clock.read)
rule record(endpoint: Id[Endpoint], post: Id[posts.Post], attempt: Int, out: Outcome, took: Time) -> Int = {
  if not (endpoint in Endpoint) { return 0; }
  let e = Endpoint[endpoint];
  let mine = Delivery.filter(it.endpoint == e.id).sort(it.seq);
  let streak = if delivered(out) then 0 else (mine.last.streak else 0) + 1;
  let status = match out { Answered(n) => Some(n); Failed(_) => none; }
  let error = match out { Answered(_) => none; Failed(text) => Some(text.take(500)); }
  Delivery.insert(Delivery(
    Id.fresh(), org: e.org, endpoint: e.id, seq: mine.size + 1, webhook: "deliver", event: post.text,
    topic: TOPIC, attempt: attempt, ok: delivered(out), status: status, took: took, error: error,
    streak: streak, at: clock.instant(),
  ));
  if streak >= DISABLE_AFTER and e.disabled == none {
    Endpoint[e.id].disabled = Some("{streak} consecutive failures");
  }
  return streak;
}

law cap_holds(t: Trace[add | remove], org: Id[orgs.Org]) =
  t.states.all(s => s.Endpoint.filter(it.org == org).size <= CAP);

law seq_counts_from_one(t: Trace[deliver | record], e: Id[Endpoint]) =
  t.states.all(s => s.Delivery.filter(it.endpoint == e).sort(it.seq).map(it.seq) == (1..=s.Delivery.filter(it.endpoint == e).size));

@given(orgs.seeded)
test refuses_unroutable = {
  for u in [url"http://169.254.169.254/latest/meta-data/", url"https://10.0.0.7/hooks", url"https://[::1]/hooks", url"https://localhost/hooks"] {
    add(orgs.ada, org: orgs.acme, url: u) -> fail UrlRefused;
  }
  assert Endpoint.size == 0;
}

@given(orgs.seeded)
test eleventh_is_refused = {
  for n in 1..=CAP { add(orgs.ada, org: orgs.acme, url: HOOK); }
  add(orgs.ada, org: orgs.acme, url: HOOK) -> fail LimitReached;
  assert Endpoint.size == CAP;
}

@given(orgs.seeded)
test revoke = {
  let e = add(orgs.ada, org: orgs.acme, url: HOOK);
  remove(orgs.bruno, org: orgs.acme, id: e.id) -> fail Forbidden;
  remove(orgs.mara, org: orgs.tinta, id: e.id) -> fail NotFound;
  remove(orgs.ada, org: orgs.acme, id: e.id) -> e.id;
  assert not (e.id in Endpoint);
  remove(orgs.ada, org: orgs.acme, id: e.id) -> fail NotFound;
}

@given(orgs.seeded)
test only_an_administrator_registers = {
  add(orgs.bruno, org: orgs.acme, url: HOOK) -> fail Forbidden;
}

@given(orgs.seeded, posts.one_published, http.reply(503))
test streak_counts_then_resets = {
  let e = add(orgs.ada, org: orgs.acme, url: HOOK);
  for i in 1..=2 { deliver(posts.hello, endpoint: e.id) -> fail Unreachable; }
  http.reply(200);
  deliver(posts.hello, endpoint: e.id);
  assert Delivery.sort(it.seq).map(it.streak) == [1, 2, 0];
}

@given(orgs.seeded, posts.one_published, http.reply(503))
test disables_once_and_stops_sending = {
  let e = add(orgs.ada, org: orgs.acme, url: HOOK);
  for i in 1..=DISABLE_AFTER { deliver(posts.hello, endpoint: e.id) -> fail Unreachable; }
  assert Endpoint[e.id].disabled == Some("{DISABLE_AFTER} consecutive failures");
  deliver(posts.hello, endpoint: e.id);
  assert http.calls.size == DISABLE_AFTER;
}

@given(orgs.seeded, posts.one_draft, http.reply(200))
test publish_delivers_signed_to_live_endpoints = {
  let a = add(orgs.ada, org: orgs.acme, url: HOOK);
  let b = add(orgs.ada, org: orgs.acme, url: url"https://hooks.example.com/postly/2");
  Endpoint[b.id].disabled = Some("gone");
  posts.publish(orgs.ada, id: posts.draft);
  tasks.drain();
  assert http.calls.map(it.url) == [HOOK];
  assert http.calls.first.topic == TOPIC;
  assert http.calls.first.verifies(a.secret);
  assert http.calls.first.body.id == posts.draft;
}

@target(ultimate)
app Webhooks(rules: [add, remove, announce, deliver], http: [add, remove]);
```

## Game movement

```slg
use game;

component Pos(at: Vec3[m]);
component Vel(v: Vec3[m/s]);
component Grounded(on: Bool);
const GRAVITY: Vec3[m/s²] = (0, -9.81, 0)[m/s²];

@on(tick(60Hz))
@after(input)
@within(0.5ms, per: 10_000)
@alloc(none)
@post(pos.at.y >= 0m)
system movement(pos: mut Pos, vel: mut Vel, g: Grounded, dt: Time) = {
  if not g.on { vel.v = vel.v + GRAVITY * dt; }
  pos.at = (pos.at + vel.v * dt).clamp_y(0m);
}

law no_tunnelling(t: Trace[movement]) = t.states.all(s => s.each(Pos).all(p => p.at.y >= 0m));

@given(spawn(Pos((0, 10, 0)[m]), Vel((0, 0, 0)[m/s]), Grounded(false)))
test falls_one_second = {
  world.run(60);
  world.one(Pos).at.y -> 5.01m ± 0.1m;
}
```

## Kubernetes service

```slg
use infra;

@replicas(2..=20)
@scale(cpu.mean(60s) > 70% -> up, cpu.mean(5min) < 30% -> down)
@cpu(0.25..=1)
@memory(256MiB..=512MiB)
@ready(http.get("/ready") -> 200)
@drain(30s)
@slo(p99(latency) <= 200ms, availability >= 99.9%)
service Api(image: Image = build("api"), @secret db: Url, port: Port = 8080);

@flow(Internet -> Gateway -> Api -> Postgres)
@deny(rest)
network Edge;

law no_direct_db = Edge.paths(Internet, Postgres).all(Api in it);

@given(cluster.sim(nodes: 3), load(500Hz))
test survives_node_loss = {
  cluster.kill(node: 1);
  clock.advance(60s);
  assert Api.errors.rate(60s) < 0.1%;
}

@target(k8s)
app Cluster(services: [Api], networks: [Edge]);
```

## PID controller

```slg
use control;

type Duty = -100%..=100%;
const KP: Float[s/rad] = 0.004[s/rad];
const KI: Float[1/rad] = 0.06[1/rad];
const KD: Float[s²/rad] = 0.00001[s²/rad];

@why("the motor must not overshoot into the end stop")
@on(every(10ms))
@within(200us)
@alloc(none)
@stop(Motor.temp > 80°C -> shutdown(Motor))
controller Speed(
  want: Float[rad/s],
  measured: Float[rad/s],
  i: mut Float[rad] = 0rad,
  last: mut Float[rad/s] = 0[rad/s],
) -> Duty = {
  let dt = 10ms;
  let e = want - measured;
  let u = KP * e + KI * i + KD * (e - last) / dt;
  let out = u.clamp(-100%, 100%);
  if out == u { i = i + e * dt; }
  last = e;
  return out;
}

plant Motor(
  w: Float[rad/s],
  temp: Float[K],
  duty: Duty,
  k: 280[rad/s]..=320[rad/s],
  tau: 40ms..=60ms,
) = { der(w) = (k * duty - w) / tau; }

law settles(r: 0[rad/s]..=250[rad/s]) =
  simulate(Speed, Motor, want: r, span: 2s).after(1s).all(it.w in r ± 2%);

law no_overshoot(r: 0[rad/s]..=250[rad/s]) =
  simulate(Speed, Motor, want: r, span: 2s).all(it.w <= r * 101%);

@target(cortex_m4)
app Drive(rules: [Speed]);
```

## Kernel page allocator

```slg
use kernel;

type Frame = Addr where self mod 4KiB == 0;
state used: Bitmap[FRAMES];

@why("boot CPU only; SMP needs a lock")
@uses(used.write, irq.off)
@fails(OutOfMemory)
@post(not old(used).test(result / 4KiB), used.count == old(used).count + 1)
@within(2us)
pub rule alloc() -> own Frame = machine {
  let word = used.words.position(it != 0xFFFF_FFFF_FFFF_FFFF) else fail OutOfMemory;
  let bit = used.words[word].invert().trailing_zeros();
  assert bit < 64;
  used.words[word] = used.words[word] bor (1u64 shl bit);
  return frame_at(word * 64 + bit);
}

@uses(used.write, irq.off)
@post(not used.test(f / 4KiB))
pub rule release(f: own Frame) = machine { used.clear(f / 4KiB); }

@uses(cpu.tlb)
pub rule flush_tlb(page: VirtAddr) = foreign asm "invlpg";

law never_shared(t: Trace[alloc | release]) =
  t.states.all(s => s.live(Frame).distinct().size == s.live(Frame).size);

law conserved(t: Trace[alloc | release]) = t.states.all(s => s.used.count == s.live(Frame).size);
```

## B-tree page and WAL

```slg
use storage;

@size(8KiB)
@endian(little)
layout Page(
  crc: u32,
  kind: u8 where self in [1, 2],
  _pad: Bytes[1],
  nslots: u16,
  lsn: Lsn,
  slots: Array[u16, nslots],
  heap: Rest,
) {
  law fits(p: Page) = 16B + p.nslots * 2B + p.heap.used <= 8KiB;
  law sorted(p: Page) = p.keys.sorted();
  law codec(p: Page) = decode(encode(p)) == p;
}

type Lsn = u64;
type Record = Put(tx: TxId, page: PageId, key: Bytes, value: Bytes) | Commit(tx: TxId);
state wal: Log[Record];
state cache: Map[PageId, Page];

@uses(wal.write, cache.write)
@guard(cache[id].has_room(key, value) else PageFull)
@post(cache[id].lsn == result, cache[id].get(key) == Some(value))
pub rule put(tx: TxId, id: PageId, key: Bytes, value: Bytes) -> Lsn = {
  wal.push(Put(tx, id, key, value));
  let lsn = wal.size;
  cache[id].insert(key, value);
  cache[id].lsn = lsn;
  return lsn;
}

@uses(wal.write, disk.sync)
@fails(DiskError)
@post(wal.durable >= result)
pub rule commit(tx: TxId) -> Lsn = {
  wal.push(Commit(tx));
  let lsn = wal.size;
  wal.sync();
  return lsn;
}

@uses(cache.write, disk.write)
@guard(cache[id].lsn <= wal.durable else WalFirst, cache[id].committed else Pinned)
pub rule evict(id: PageId) = machine {
  disk.pwrite(cache[id], at: id * 8KiB);
  cache.remove(id);
}

law durable(t: Trace[put | commit | evict | crash]) =
  t.states.all(s => s.wal.committed.all(it in recover(s.wal.disk)));
```

## Limits

All designer-written. None has run, so an example may hold a logic error that a test would find at once. The PID gains were picked with a throwaway Python simulation of the plant (nine corners of `k` and `tau`, six set points, 10 ms steps); that run is not the law being checked, and the script is not in this repo.
