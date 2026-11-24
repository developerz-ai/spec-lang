# Corpus

> **Design.** Nothing here runs yet. What reading real codebases says about where code mass comes from, five shapes rewritten in card syntax, and which gaps that closed or left open.

Ten private repositories, several languages and domains, described by generic shape only: sizes rounded hard, some cases merged, domain rules left out. Sizes are measured; **shares and savings are one reader's judgement**. Nobody has compiled a sketch.

## Method

|Quantity|How|Caveat|
|---|---|---|
|Lines, files|`git ls-files`, a filename pattern per feature, newline counts; code and config only|misses lines in shared files; sometimes catches a neighbour|
|Code, comments|`tokei`|tests apart|
|Tokens|bytes / 4|an estimate, not a tokenizer|
|Copying|trimmed lines, vendor names normalised, multiset overlap|ignores order, so an upper bound|
|Essential share|reading|no second rater; search was *for* bloat|

A line is **essential** when changing it changes what a user, operator or attacker can observe and nothing else implies it: a key, a legal transition, who may act, a vendor's field, a refusal's wording. **Mechanical** lines follow from such a fact: mapping, ports, wiring, re-validation, scoping again, retry ladders.

Millions of lines were scanned by pattern. The cases hold about 0.6M lines, 6-7M tokens; subsystems 7 and 8 are well over half. Case 1 was read fully, 2 and 6 substantially, the rest in part or sampled.

## Cases

Essential share is of production code, comments and tests excluded.

|#|Shape|Size|Share|Where the mass is|
|---|---|---|---|---|
|1|one opinion per person per item; set, clear, list|~2k lines, ~50 files touched|10-20%|one fact restated at nine hops; owner predicate in each query|
|2|one saved document, versioned, edited optimistically|~5k|15-25%|version check in query, service, error map, transport and client; hand-rolled rebase queue|
|3|issue, list, revoke a scoped credential and its descendants|~6k|25-35%|real security rules inside parsing and serialising|
|4|an invitation-like record: pending to one of three ends, resendable|~10k|15-25%|guarded updates, lost-race re-reads, sweeps, three surfaces|
|5|a machine asks a person; the answer resumes it|~10k|20-30%|lifecycle as strings; dedupe per origin; orphan repair|
|6|delete rows past an age, in batches (a family of jobs)|~4k|8-15%|schema, processor, builder, scheduler, registration for each|
|7|offer operations that already exist on more surfaces|100k+|25-40%|parse, scope, call, serialise, audit, map errors, again; the share is mostly prose for callers|
|8|enforce invariants the host language cannot state|200k+|30-45%|hundreds of source scanners and their tests|
|9|one client on two native platforms|~20k|35-50%|protocol, auth and stream fold written per platform|
|10|adapters for several providers of one service|~40k, two families|35-50%|token refresh, locking, errors, normalising, fakes, per provider|
|11|deliver a document on a schedule, at most once|~30k|40-55%|much is real decision; the rest is audit and missed-run repair|
|12|an action one person asks for and another confirms|~10k|25-40%|lifecycle, dedupe, per-kind payload, error catalogue|
|13|administer grants over either of two databases|~10k|15-25%|repository per dialect, DTO, audit wrapper|
|14|modules forked into a second service|~40k|25-40% per copy|kept in step by hand|
|15|dozens of services with probes, limits, network rules|~30k config|10-20%|one shape written per service|

Findings behind the table:

- **Tests outweigh production code in every case that has tests**, from about even to nearly four times. Most assertions read were meaningful; harness repeats, and the same rule at each layer.
- **Copying is concentrated.** Across five adapters, token managers match four fifths or more of their code lines after normalising names; response handlers and record parsers a quarter to three quarters, because vendors differ there. About three quarters of same-named lines in the forked modules are identical. Half of deployment-manifest lines appear verbatim in at least half of the manifests.
- **Copying drifts.** Two decoders of one protocol disagree.
- **Reasons have no checked home.** In the largest tree over a third of code-plus-comment lines are comments: reasons, incidents, promises.
- **A declarative framework did not finish the job.** Case 12 already declares input, policy, audit and cache tags in one place per action. Lifecycle, dedupe, execution and errors sit outside: still about 10k lines.
- **One result against us.** The two dialects of case 13 share under half of their repository lines.

## Recurring causes

Seen: cases of 15 where it was judged a main contributor. Coarse; overlapping.

|Cause|Seen|Card's answer|
|---|---|---|
|the same rule re-tested per layer|13|rules called directly; handlers replace mocks|
|one contract restated per layer|12|`type`, `entity`; clients generated|
|retry, idempotency, locks, races by hand|10|`@retry`, `@idempotent`, atomic `rule`, resumable `task`|
|wiring, ports, registries|9|effects plus `app`|
|owner scoping and roles repeated|9|`@tenant`, `@access`|
|error classes and their translation|9|typed failures, one meaning everywhere|
|lifecycle as strings and conditional updates|7|`flow` with `@edge`|
|copied adapters, clients, forks|7|targets; vocabulary kinds: **partial**|
|one operation on several surfaces|5|`@on`: **partial**|
|external source scanners|1, very large|effects, ids, single declaration; packages add notes|
|repeated configuration|2|**open**: bundling|

By filename alone, about 40% of scanner lines look statable as a type, effect or single-declaration rule. A language plausibly removes 30-50% of that tree.

## Five sketches in card syntax

They parse; their types, names and behaviour are unchecked. Notes and kinds outside the card (`@page`, `Date`, `Token`, `blob`, `decimal`) are assumed vocabulary; `t1` and `boss` (an admin) are test constants, and `admin` is a subject `fn` ([27](27-authorization.md)). No UI, copy, history or vendor tests: length is **not comparable** with the sizes above.

### A. Owned fact (cases 1, 3, 13)

```slg
use web;

type Vote = Up | Down;
type Key(by: Id[User], item: Id[Item]);
type Rating(vote: Vote, at: Time);
state ratings: Map[Key, Rating];
state viewers: Map[Id[Item], Set[Id[User]]];

@why("one current opinion per person per item; unseen and absent look alike")
@on(http.put("/items/{item}/rating"))
@access(by in viewers[item])
@uses(ratings.write, viewers.read, clock.read)
pub rule rate(by: Id[User], item: Id[Item], vote: Vote) = {
  ratings[Key(by, item: item)] = Rating(vote, at: now());
}

@access(true)
@uses(ratings.read)
@page(50)
pub query mine(by: Id[User]) -> List[Rating] =
  ratings.entries.filter(it.key.by == by).map(it.value).sort(it.at).reversed();

@given(viewers[Id(7)] -> Set(Id(1)), clock.at(0s))
test rating_replaces = {
  rate(Id(1), item: Id(7), vote: Up);
  rate(Id(1), item: Id(7), vote: Down);
  mine(Id(1)) -> [Rating(Down, at: 0s)];
  rate(Id(2), item: Id(7), vote: Up) -> fail NotFound;
}
```

**Goes:** storage, upsert, parsing, client, scoping, paging, wiring, layer tests. **Stays:** controls, copy, caller-facing prose. **Open:** the same rule as an agent tool with description and example. `@why` is a reason, not instructions.

### B. Lifecycle (cases 4, 5, 12)

```slg
use mail;

type Status = Pending | Accepted | Revoked | Expired;
type Invite(@pii email: Text, @secret token: Token, status: Status = Pending);
state invites: Map[Id[Invite], Invite];
state members: Set[Text];

@edge(Pending -> Accepted, via: accept)
@edge(Pending -> Revoked, via: revoke, by: admin)
@edge(Pending -> Pending, via: resend, by: admin)
@edge(Pending -> Expired, after: 7d)
flow InviteLife(at: Invite.status);

@access(true)
@uses(invites.write, members.write)
@guard(invites[id].token == token else NotFound)
@guard(invites[id].status == Pending else Gone)
pub rule accept(id: Id[Invite], @secret token: Token) = {
  invites[id].status = Accepted;
  members.insert(invites[id].email);
}

@access(admin(by))
@uses(invites.write)
@guard(invites[id].status == Pending else Gone)
pub rule revoke(by: Id[User], id: Id[Invite]) = { invites[id].status = Revoked; }

@access(admin(by))
@uses(invites.write, random.token, mail.send)
@guard(invites[id].status == Pending else Gone)
pub rule resend(by: Id[User], id: Id[Invite]) = {
  invites[id].token = random.token();
  mail.send(invites[id].email, body: invites[id].token); // after commit, at least once
}

@given(clock.at(0s), invites[Id(1)] -> Invite("a@example.com", token: t1))
test pending = { assert members.size == 0; }

@given(pending)
test revoked_is_dead = {
  revoke(boss, id: Id(1));
  accept(Id(1), token: t1) -> fail Gone;
}

@given(pending)
test lapses = {
  clock.advance(7d);
  accept(Id(1), token: t1) -> fail Gone;
}

@given(pending)
test race(t: Trace[accept | revoke | resend | crash]) = {
  for s in t.states { assert s.members.size <= 1; }
}
```

**Goes:** conditional updates, lost-race repair, the expiry poll, outbox. **Stays:** template, transport, who is an admin. **Open:** row 5 below.

### C. Retention (case 6)

```slg
use files;

entity Notice(id: Id[Notice], at: Time);
entity Export(id: Id[Export], file: Blob, at: Time);

rule purge(n: Int) = {
  for x in Notice.filter(it.at + 90d <= now()).take(n) { Notice.remove(x.id); }
}

@why("read within weeks or never")
@on(every(6h))
@within(30s, over: stop)
task sweep_notices() = {
  while Notice.any(it.at + 90d <= now()) { purge(500); }
}

rule forget(id: Id[Export]) = { Export.remove(id); }

@why("the file goes first, so a crash leaves a row that still names it")
@on(every(6h))
@within(30s, over: stop)
@uses(blob.delete)
task sweep_exports() = {
  for x in Export.filter(it.at + 1d <= now()).take(500) {
    blob.delete(x.file) else fail Unreachable;
    forget(x.id);
  }
}

@given(clock.at(0s), Notice[Id(1)] -> Notice(Id(1), at: 0s))
test keeps_the_young = {
  clock.advance(89d);
  assert Id(1) in Notice;
  clock.advance(2d);
  assert not (Id(1) in Notice);
}
```

**Goes:** queue, payload schema, builder, scheduler, registration, resume. **Stays:** windows, order, what failure means. **Open:** stating the family once (row 1). `task` made batches and file-before-row expressible.

### D. Versioned document (case 2)

```slg
type Conflict(now: Doc);
type Pane(item: Id[Item], width: 1..=12);
type Layout = List[Pane] where self.map(it.item).distinct().size == self.size;
type Doc(id: Id[Doc], owner: Id[User], layout: Layout, version: Int = 0);
state docs: Map[Id[Doc], Doc];

fn clash(a: Layout, b: Layout) -> Bool = a.any(p => b.any(q => q.item == p.item));

@access(docs[id].owner == by)
@uses(docs.write)
@guard(docs[id].version == base else Conflict(docs[id]))
@guard(not docs.values.any(it.owner == by and it.id != id and clash(it.layout, layout)) else Held)
pub rule save(by: Id[User], id: Id[Doc], base: Int, layout: Layout) -> Doc = {
  docs[id] = docs[id].with(layout: layout, version: base + 1);
  return docs[id];
}

@why("retry silently only when nobody else moved anything")
@cases((1, 2, 1) -> Some(2), (1, 2, 3) -> none)
fn rebase(base: Int, mine: Int, theirs: Int) -> Option[Int] =
  if theirs == base then Some(mine) else none;

law versions_never_fall(t: Trace[save], id: Id[Doc]) =
  t.steps.all(id in it.before and id in it.after
    implies it.after[id].version >= it.before[id].version);
```

**Goes:** version predicate, verdict read, error translation. **Stays:** drag, rendering, the merge rule (integers stand in). **Open:** the client half (row 3). The cross-document guard hides its cost: serialising an owner's writes.

### E. Provider adapter (case 10)

```slg
use http;

type Item(kind: Text, amount: Text, day: Text, code: Text);
type Entry(amount: Decimal, day: Date, code: Text[1..=64]);
type Days(from: Date, to: Date);
type Place(link: Id[Link], code: Text);
state bearers: Map[Id[Link], Token];
state ledger: Map[Place, Entry];

fn sign(kind: Text) -> Int = if kind == "in" then 1 else if kind == "out" then -1 else fail Unreadable;

@fails(Unreadable)
@cases(
  Item("in", amount: "1.50", day: "2026-01-02", code: "a")
    -> Entry(1.50, day: date"2026-01-02", code: "a"),
  Item("??", amount: "1.50", day: "2026-01-02", code: "a") -> fail Unreadable,
)
fn entry(i: Item) -> Entry =
  Entry(sign(i.kind) * (decimal(i.amount) else fail Unreadable), day: parse_day(i.day), code: i.code);

rule keep(link: Id[Link], found: List[Entry]) = {
  for e in found { ledger[Place(link, code: e.code)] = e; }
}

@why("each window commits alone; a crash resumes at the next one")
@retry(4, backoff: 1s, on: Unreachable)
@uses(http.call, bearers.read)
task sync(link: Id[Link], windows: List[Days]) = {
  for w in windows {
    let reply = http.call("/entries", bearer: bearers[link], days: w) else fail Unreachable;
    if reply.status >= 500 { fail Unreachable; }
    keep(link, found: reply.items.map(entry(it)));
  }
}

const day: Days = Days(date"2026-01-01", to: date"2026-01-02");

@given(http.reply(503), bearers[Id(1)] -> t1)
test outage_is_typed = {
  sync(Id(1), windows: [day]) -> fail Unreachable;
  assert ledger.size == 0;
}
```

**Goes:** transport, error classes, fake server, a second-language copy. **Stays:** field map, sign rules, vendor quirks. **Open:** row 4, and a typed decoder for `reply`. Adapters plausibly keep a third to a half of their code.

## What the exercise changed

The first draft of these sketches could not be written. The card then changed. Closed means a form parses, not that it works.

|Closed gap|Cases|Now on the card|
|---|---|---|
|calls that need a reply, inside an atomic rule|5, 7, 9-11|`task`; a `rule` sends after commit and never waits|
|work longer than one commit|6, 10-12|`task`: each rule commits alone, finished steps do not rerun|
|raising an event|4, 5, 11, 12|`event`, `.emit`, `@on(event)`|
|edges without actor, deadline or rule|4, 5, 11, 12|`@edge(A -> B, via:, by:, after:)`|
|undefined access subjects|1-5, 12, 13|`anyone`, `user`, a role, a field holding an `Id[User]`|
|refusals leaking existence|1-5, 13|exposed `Missing` and denial are both `NotFound`|
|budget overrun|6, 11, 15|`over: stop` or `fail`; default reports|
|guard bindings in later guards|2, 4|in scope after the guard|

|#|Still open|Cases|Smallest change|
|---|---|---|---|
|1|**Note bundling.** A family of retentions or services repeats a note set; a `note` body is never shown|6, 13, 15|let a `note` body be a list of notes|
|2|**Prose for callers** and per-surface projection; most of case 7's essential share|7|a `@doc` note and projection policy in a vocabulary|
|3|**Client speculation and merge.** Nothing says a rule may run ahead on a device, in order, then reconcile with `rebase`|2|a vocabulary note with a stated model|
|4|**Reserved words as field names.** `ref` is common in foreign payloads and failed here first|10|make `own ref mut` words only inside `machine`; let a decoder map names|
|5|**Edge details.** Whether a self-edge restarts `after:`; the error of an illegal move; whether `by:` replaces `@access`, as B writes both|4, 5, 12|say it|
|6|**Subjects derived from data** (a viewer set, a token holder) are still guards|1, 4|named predicates over `by`|
|7|**Direct calls in tests.** Whether an unexposed call sees `Missing` or `NotFound`|1, 4|say it|

The riskiest new claim is `task`: a retried call is only as safe as the third party's handling of its key, which is *assumed*.

## What this orders

1. Typed entity, rule and query projected to storage, wire, client and tools: largest mass.
2. Tests beside rules with clock, net and crash handlers: tests outweigh code everywhere.
3. `task`, `event`, `flow` and delivery notes: fewest lines, most danger.
4. Typed failure kept whole across layers; bundled notes (retention, service shape): cheap, common, a good first demo.
5. Adapter vocabulary: large mass, hardest to make honest.

Not early: a general UI language. Presentation was not shown to be mechanical.

## Counterexamples

|Code|Why a language would not shrink it|
|---|---|
|deterministic fixed-step simulation (about 1.5k lines with its maths)|already algorithm; ordering and overflow *are* the content. The gain is checking, not size|
|admission policy over a third-party syntax tree (a few hundred lines)|each branch blocks one thing; its tests are the specification|
|translation between several vendor wire formats (thousands of lines)|shared parts already factored; the rest is lossy streaming difference|
|a finite decision table (tens of lines)|good TypeScript does this today|

## Limits

- **Judged shares.** One reader, partial reading; unread code inherits the ranges.
- **One organisation.** Agent-written: small files, many guards, heavy comments. Some size is style.
- **Selected for bloat.** No base rate follows.
- **Fork and template.** Much of 14 and 15 needs a package or a template, not a language.
- **Edges cost.** Some mechanical mass is hard-won behaviour a sketch drops silently. Parsing proves little.
- **Settling it.** Port 1, 4 and 6 whole; use existing tests as black-box oracles; count what is added back; add a blind second rater. Kill rule: if under half of production tokens go with behaviour kept, drop the first row of the order. See [19-density-and-experiments](19-density-and-experiments.md).
