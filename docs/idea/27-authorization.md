# Authorization

> **Design.** Nothing here runs yet. This doc defines the one authorization model of spec-lang: who the caller is, how "may this caller do this" is written, what it means at run time and in tests, and what every backend must project.

**Rule: `@access(p else Err)` is the only way to say who may call. `p` is a Bool over `by` and the parameters. Where there is a caller and no `@access`, the call is denied.** Syntax is the [language card](01-language-card.md). Labels and information flow are [05-effects-and-trust](05-effects-and-trust.md); exact evaluation order is [30-type-rules](30-type-rules.md).

## The model in one table

| Piece | Is | Not |
|---|---|---|
| caller | `by: Id[User]`, the first parameter of anything that can be called from outside | a session, a token, a request |
| subject | a `fn` returning `Bool` whose first parameter is `by` | a built-in role, a string, a group in a config file |
| access | `@access(p else Err)` on a `rule`, `query` or `task`: may this caller call at all | "is the call legal now", which is a `@guard` |
| row access | `@access(subject -> ops, ...)` on an `entity` | a filter the author writes into each query |
| field access | the same note on one field | a serializer option |
| tenancy | `@tenant(x)`: a label, checked as information flow | an access rule |
| move access | `by:` on a flow `@edge` | a guard inside the rule |

Authentication, meaning how a request is turned into a `by`, is not in the language. The backend does it. Everything after that is.

## Subjects

A subject is an ordinary `fn`. It reads state, writes nothing, calls nothing outside, and takes `by` first. Roles are data plus a subject; a hierarchy is one subject calling another.

```slg
type Kind = Person | Agent(owner: Id[User]) | Key(owner: Id[User]);
type User(id: Id[User], org: Id[Org], kind: Kind, admin: Bool = false);
state users: Map[Id[User], User];

fn member(by: Id[User]) -> Bool = by in users;
fn admin(by: Id[User]) -> Bool = member(by) and users[by].admin;
fn person(by: Id[User]) -> Bool = member(by) and users[by].kind == Person;
fn acts_for(by: Id[User], u: Id[User]) -> Bool =
  by == u or users[by].kind.is(Agent(u)) or users[by].kind.is(Key(u));
```

| Rule | Reason |
|---|---|
| a subject is total: inside `@access`, a `Missing` makes the predicate false | an absent row and a denied row must look the same |
| a subject may be `foreign` | existing permission code can be reused; its evidence starts at `assumed` and the access matrix says so |
| subjects compose with `and`, `or`, `not` | no second policy language |
| there is no superuser | every "may" is a subject somebody wrote and a test can call |

## Access on rules, queries and tasks

Order of evaluation for a call that arrives from outside:

```text
1. authenticate            backend; yields by, or refuses before the language is involved
2. decode parameters       against their types; failure is a decoding error, never a body run
3. @access                 false, or Missing while evaluating it  ->  Err (default NotFound)
4. flow edge `by:`         false                                  ->  Forbidden
5. @guard clauses, in order
6. body, atomically
```

- Steps 3 to 6 read one consistent state: the check and the write are in the same commit, so there is no gap between them.
- A denied call changes nothing and emits nothing.
- **A guard that mentions `by` is not access.** It is legal (`@guard(votes[by] == none else AlreadyVoted)`), but a declaration someone can call in to (a calling trigger, or a place on an `app` surface) and no `@access` is denied whatever its guards say: it gets no route, tool or client. A declaration nobody can call in to, like the card's `hold`, needs none, and a direct call then has no access to check. The split is what lets a reviewer read one note to learn who may call.
- A private rule called by another rule joins its caller. If it has a `by` parameter and its own `@access`, that is checked with the `by` it is given. Access is part of what the rule means, not part of the transport.

## Access on entities and fields

```text
@access(owner -> all, shared -> read, admin -> read)

left   a field of type Id[User]  (means row.field == by)
       or a subject taking (by) or (by, row)
right  read | create | update | remove | write (the last three) | all
```

| Effect | Mechanism |
|---|---|
| derived operations of the entity | each gets the matching `@access`; nothing is exposed that no clause allows |
| any `query` with a caller that reads the entity | rows the caller may not read are **absent** from the result: counts, pages and sort positions are computed after the filter |
| any `rule` with a caller that writes a row | a row the caller may not write behaves as `Missing` |
| a hand-written `pub` declaration | still needs its own `@access`; the entity's clauses are and-ed with it. Access narrows, never widens |

**Field access is a label, not a filter.** `@access(owner -> read)` on a field gives its values a visibility label. Like `@pii` and `@secret`, it joins through expressions and is checked at every exit. A derived read returns the record without fields the caller may not see. A hand-written query that would return such a field to a caller who may not see it does not type-check; it returns a narrower type instead. A reply's shape never silently depends on who asked.

## Tenancy is not access

| | `@tenant(x)` | `@access` |
|---|---|---|
| question | where may this data flow | who may make this call |
| kind of claim | run-pair (no interference between tenants) | value claim checked at run time |
| checked | by label typing, before running | at step 3, every call |
| best evidence | `proven`, with transfer to the backend | `proven` that the check runs; whether the predicate is the right one is tests and laws |
| can `admin -> all` cross it | no: `all` means all rows of the caller's tenant | not applicable |
| way out | a listed declassification | another clause |

They compose: tenancy narrows the store to the caller's tenant, then access decides inside it.

## Moves: flow `by:`

`@edge(Published -> Archived, via: archive, by: admin)` adds `admin(by)` to what `archive` requires **for that move**. The source state picks the edge. If one rule can make several moves from the same state with different `by`, the edge is not determined at entry and the compiler reports `E-OPEN`. Failing an edge's `by` is `Forbidden`, not `NotFound`: the caller already passed the rule's `@access`, so existence is known to them.

## Callers that are not people

| Caller | Model |
|---|---|
| agent, API key, service account | a row in `users` with its own `Id[User]`; what it may do for someone is a subject such as `acts_for`. Scopes are data that a subject reads |
| delegation | never implicit. `acts_for(by, row.owner)` in the `@access` of each rule that allows it; a rule that must be done in person says `person(by)` |
| anonymous | an exposed declaration with no `by` needs `@access(true)`, written out. Mixed surfaces take `by: Option[Id[User]]` |
| timer, event, interrupt, flow deadline | **no caller.** The rule has no `by`, needs no `@access`, and cannot be exposed |
| a caller-less rule calling a rule that takes `by` | it passes an `Id[User]` taken from data, and access is checked for that user **at that moment**. Rights revoked since the event was emitted are gone |
| a `task` | `by` is fixed at start and journalled; every rule the task calls checks access again in its own commit, so revocation takes effect between steps |

The audit trail records `by` as it was: the agent's id, not its owner's. An agent's tool list is derived: the exposed declarations whose `@access` can be true for a caller of its kind.

## NotFound, Forbidden and existence

| Situation | Reply |
|---|---|
| `@access` false | `NotFound`, unless the note says `else Forbidden` |
| `Missing` raised anywhere in an exposed rule | the same `NotFound`, same shape |
| edge `by:` false | `Forbidden` |
| row not readable, in a query | absent |

Use `else Forbidden` only where existence is already public to that caller. The compiler adds one report, the **existence oracles**: every guard error and every uniqueness failure whose outcome depends on a row the caller may not read ("email already taken" is the usual one). For `@tenant` data such a dependence is a label error and blocks. For access-only data it is listed for a human to accept, because sometimes it is intended.

## Tests

**A direct call in a test checks `@access`.** `by` is an ordinary argument and access is part of the rule's meaning, so there is nothing to bypass and no "as admin" switch.

| In a test | Behaviour |
|---|---|
| direct call, access false | fails with the note's error |
| direct call, key missing | fails with `Missing`: the collapse into `NotFound` is a projection of exposure, not of the rule |
| call through the trigger (`http.post(...)` in the scenario) | the exposed behaviour, collapse included |
| a caller-less rule | called with no `by` |
| every exposed declaration | a generated property: for any `by` where the predicate is false, the reply is the access error and the state is unchanged |
| `spec-lang view --as access` | a subject-by-declaration matrix, rendered for human approval; a change to it is a diff a person signs |

## What a backend must project

1. Turn its own authentication into `by`; refuse before step 2 if it cannot.
2. Expose nothing without an `@access`. A route, a tool or a client stub exists only for declarations that have one.
3. Evaluate access in the same transaction as the body.
4. Give identical replies, status and shape for `NotFound` from access and from `Missing`.
5. Push row access into the store's query, never filter after paging.
6. Project fields by the caller's visibility on every reply, live update and export.
7. Key every cache and live subscription by what the caller may see, and re-evaluate when a subject's inputs change.
8. Treat the client as untrusted: hiding a button is a courtesy; the server decides.
9. Record `by`, the declaration and the outcome for each exposed call.

Each item is covered by parity tests ([11-backends-and-parity](11-backends-and-parity.md)), so its evidence is reported as transfer: usually `sampled`.

## A worked module

```slg
use web;

type Status = Draft | Published | Archived;

fn shared(by: Id[User], d: Doc) -> Bool = by in d.readers;
fn agent(by: Id[User], d: Doc) -> Bool = acts_for(by, d.owner);

@tenant(org)
@access(owner -> all, agent -> update, shared -> read, admin -> (read, update))
entity Doc(
  id: Id[Doc],
  org: Id[Org],
  owner: Id[User],
  readers: Set[Id[User]],
  title: Text[1..=200],
  @pii body: Text,
  @access(owner -> read) notes: Text = "",
  status: Status = Draft,
);

@edge(Draft -> Published, via: publish)
@edge(Draft -> Archived, via: archive)
@edge(Published -> Archived, via: archive, by: admin)
@edge(Draft -> Archived, after: 365d)
flow DocLife(at: Doc.status);

@uses(Doc.write)
@access(acts_for(by, Doc[id].owner))
pub rule publish(by: Id[User], id: Id[Doc]) = { Doc[id].status = Published; }

@uses(Doc.write)
@access(acts_for(by, Doc[id].owner) or admin(by))
pub rule archive(by: Id[User], id: Id[Doc]) = { Doc[id].status = Archived; }

@why("handing out access is done in person, never by a key or an agent")
@uses(Doc.write)
@access(Doc[id].owner == by and person(by))
@guard(member(to) else NoSuchUser)
pub rule share(by: Id[User], id: Id[Doc], to: Id[User]) = { Doc[id].readers.insert(to); }

@uses(Doc.read)
@access(member(by))
pub query visible(by: Id[User]) -> List[Doc] = Doc.filter(it.status != Archived).sort(it.title);

const ana: Id[User] = Id(1);
const bot: Id[User] = Id(2);
const eve: Id[User] = Id(3);
const d1: Id[Doc] = Id(1);

@given(
  users[ana] -> User(ana, org: Id(1), kind: Person),
  users[bot] -> User(bot, org: Id(1), kind: Agent(ana)),
  users[eve] -> User(eve, org: Id(1), kind: Person),
  Doc[d1] -> Doc(d1, org: Id(1), owner: ana, readers: Set(), title: "Plan", body: "draft"),
)
test who_may = {
  visible(eve) -> [];
  publish(eve, d1) -> fail NotFound;
  share(bot, d1, to: eve) -> fail NotFound;
  publish(bot, d1);
  archive(ana, d1) -> fail Forbidden;
  share(ana, d1, to: eve);
  visible(eve).map(it.title) -> ["Plan"];
}

law only_readers_see(t: Trace[share | publish | visible], by: Id[User]) =
  t.steps.all(s => s.event.is(visible(by)) and s.reply.is(Ok(docs)) implies
    docs.all(d => d.owner == by or by in d.readers or s.before.users[by].admin));
```

| Line of the test | Why |
|---|---|
| `visible(eve) -> []` | Eve is a member, so the call is allowed; the row is not readable by her, so it is absent |
| `publish(eve, d1)` fails `NotFound` | Eve does not act for Ana; she learns nothing about `d1` |
| `share(bot, ...)` fails `NotFound` | the agent acts for Ana, but `share` demands a person |
| `publish(bot, d1)` succeeds | delegation, stated in that rule's `@access`; the entity's `agent -> update` lets the write through, and gives the agent no read |
| `archive(ana, d1)` fails `Forbidden` | Ana passes the rule's access; the `Published -> Archived` edge needs `admin` |
| the 365-day edge | no caller, no access; it is a deadline of the flow |

What the author did not write: a route guard, a query filter, a serializer field list, a policy file, a tool manifest, or a test helper that logs in.

## Evidence

| Claim | Expected |
|---|---|
| no exposed declaration lacks `@access` | `proven` (structural) |
| a denied call changes nothing | `proven`: access runs before the atomic body |
| tenant isolation | `proven` by labels; transfer `sampled` |
| a field never reaches a caller who may not see it | `proven` by labels; transfer `sampled` |
| the predicate is the policy the business wants | tests, laws and the approved access matrix; never more than that |
| a `foreign` subject | `assumed`, or `sampled` with a differential test |

## Limits: threats this does not cover

- **Entity clauses and a rule's own `@access` are and-ed.** Safe by default, but an entity clause has to be wide enough for every hand-written rule, so `admin -> update` also opens derived updates. The other choice, a rule's clause deciding its own writes, is open.
- **Authentication.** Credential theft, weak sessions, token replay, cross-site request forgery and key storage belong to the backend and its framework.
- **The wrong predicate.** A correctly enforced bad policy is still bad. The access matrix makes it reviewable; it does not make it right.
- **Timing and size channels.** `NotFound` from access and from a missing row may take different time. Counts after filtering can still leak through pagination behaviour under load.
- **Operators.** Anyone with direct access to the store, the logs or the host bypasses everything here.
- **Inference.** A caller allowed to read aggregates may reconstruct rows. Listed existence oracles are accepted by a human, not eliminated.
- **Abuse within rights.** Rate limits, quotas and brute force are policies of a vocabulary, not of `@access`.
- **Stale rights.** Revocation takes effect at the next commit. A reply already sent, a client cache or a live subscription not yet re-evaluated holds old data for a time the backend determines.
- **Delegation chains.** `acts_for` is one hop here. Longer chains are ordinary data, and nothing proves they terminate where intended.
- **Subjects that read state** make the access matrix depend on data; where a subject is not decidable statically, the matrix shows "depends on" rather than yes or no.
