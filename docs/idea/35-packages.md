# Packages

> **Design.** Nothing here runs yet. How ideas are shared: what a package is, where it lives, how it is trusted.

**A package is source with its tests and laws. Adding one runs its tests inside the engine, which can reach nothing, and runs nothing else**, so what usually makes a dependency dangerous at install time does not exist. What remains is whether its logic is right, and that is what the laws are for.

## What can be shared

| Kind | Example | Why it is worth sharing |
|---|---|---|
| logic | invites, seats and quotas, double-entry ledger, rate limits | the idea with its edge cases already tested |
| vocabulary | `web`, `jobs`, a bank-statement vocabulary | new kinds and notes ([09](09-vocabularies.md)) |
| contract | the interface of a payment provider | what is known and what is `assumed` |
| **simulator** | a handler that behaves like that provider: late, twice, refused | every user's local tests get closer to reality |
| laws | "what any ledger must satisfy", without an implementation | checks for your own version |
| data structure | interval tree, rope | with its `@steps` claims |
| backend | Ultimate, Rails | a full program, with a parity record ([11](11-backends-and-parity.md)) |

Simulators may matter most. Somebody learns once how a provider misbehaves, and no one else has to.

## Where it lives

Git, no central registry at first. A package is a repository and a tag; the lock file pins a commit and a content hash.

```text
$ spec-lang edit --add github.com/example/ledger@v2
  adds      ledger: 3 types, 4 rules, 6 laws, 21 tests      11 declarations, 1,940 tokens
  effects   none
  trusts    nothing outside the language
  tests     21 pass here
  lock      spec-lang.lock: ledger v2, commit, hash
```

| Choice | Why |
|---|---|
| git and tags | exists, mirrors, works offline, private repositories work the same way |
| content hash decides | a moved tag or a changed host cannot change what is built |
| no install scripts, no build hooks | nothing in a package executes until a rule you call is run, in the engine |
| vendored copy allowed | a small idea can simply be copied in; the hash still says where it came from |
| an index later | search is a web page over public tags; never the place code is fetched from |

## Using one

A package's `pub` part is its contract; bodies stay private. It owns its state. It reaches the world only through effects that its signature shows and your `app` grants.

```slg
use ledger;

@idempotent(request)
@access(clerk(by))
pub rule pay(by: Id[User], request: Id[Request], from: Id[ledger.Account], to: Id[ledger.Account], amount: ledger.Cents) = {
  ledger.transfer(request, from: from, to: to, amount: amount);
}

@given(ledger.two_accounts)
test paying_moves_money_and_keeps_the_total = {
  let before = ledger.total(ledger.USD);
  pay(ana, request: r1, from: ledger.a, to: ledger.b, amount: 30);
  pay(ana, request: r1, from: ledger.a, to: ledger.b, amount: 30);
  assert ledger.total(ledger.USD) == before;
}
```

The caller passes the request key on. A key made inside `pay` would be new on every retry, and the money would move twice.

## Trust

| Question | Answered by |
|---|---|
| does it do what it says | its laws and tests, **rerun on your machine**, not a badge |
| what can it touch | its effects; a pure package has none |
| did an update change behaviour | the version step is computed: a removed or weakened law, a new effect or a new assumption is major |
| what exactly changed | a guarantee diff, approved like any other ([08](08-approval-and-anti-cheating.md)) |
| is the law worth anything | vacuity and mutation checks, shown beside it |
| who wrote it | signed tags; a name is a claim, and the hash is the identity |
| can its text steer my agent | names and `@why` text are untrusted data, quoted and bounded ([05](05-effects-and-trust.md)) |

An update that only adds a law is not automatically safe: your own use may break it, and the check says so before anything lands.

## Versions

One version of a module per program, because a module owns state and types must match. A conflict is a question with both paths, never a silent pick. Two versions may run side by side only behind a boundary type.

## For agents

`view --find "seat limits"` searches what is already in the lock file, then indexes, and returns contracts, laws and size, not source. An agent reads a package's contract in a few hundred tokens and never loads its body. Publishing is a tag on a commit that passed `land`.

The habit to keep: **reuse an idea when its laws say what you need; write it when they do not.** A short idea is cheaper to own than to depend on.

## Limits

- Laws describe what their author thought of. A package can be green and wrong for you.
- Rerunning everyone's tests costs time; proofs are cached by hash, and a cached proof trusts whoever made the cache unless it is replayed.
- Git hosts can vanish. A vendored copy or a mirror is the answer, and someone must keep one.
- Backends and `foreign` code do run as normal programs. For those, the ordinary dangers of a dependency return, and they are listed apart.
- Sharing a module that works over **your** types, such as webhooks for any kind of organisation, needs parameters on modules. The grammar has none; packages with fixed types are what this design can do today.
- Nobody has published anything, so discovery, naming disputes and abandoned packages are untested.
