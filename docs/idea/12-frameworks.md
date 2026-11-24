# Frameworks

> **Design.** Nothing here runs yet. This doc covers Ultimate as the first backend: the mapping, one real feature measured, how a module lives inside an existing app, and an ordinary Tuesday.

**Rule: lower to the framework, and delegate to it everything it already derives.** The author writes logic and tests in one `.slg` file. The backend emits ordinary framework code. The framework keeps doing what it does well. The contract every backend signs is in [`11-backends-and-parity.md`](11-backends-and-parity.md); the decision is [`003`](../decisions/003-frameworks-are-backends.md).

## What was measured

The feature is outbound webhooks in Postly, the reference app in the Ultimate repository (`examples/dummy`, v31.22.3): an org registers up to ten receivers, a published post is delivered signed to each live one, every attempt is recorded, and ten failures in a row disable the endpoint. It is bounded (one directory plus two schema files) and cuts across entities, authorisation, actions, a job, errors and tests. It has no screen.

The spec-lang version is the [webhooks example](24-examples.md#webhooks-tasks-retries-signing), called `webhooks.slg` below. It parses with the grammar. Nothing else about it is checked.

| | Lines | Tokens (o200k) |
|---|---|---|
| TypeScript source, 11 files, comments stripped | 352 | 3,584 |
| Registration lines in four shared files | 27 | 258 |
| TypeScript tests, 3 files, comments stripped | 252 | 2,863 |
| **TypeScript total** | **631** | **6,705** |
| `webhooks.slg`: types, errors, rules, one task, eight `@why` lines | 161 | 1,677 |
| `webhooks.slg`: two laws, seven tests | 67 | 832 |
| **spec-lang total** | **228** | **2,509** |

**Ratio: 2.7x tokens, 2.8x lines.** Source and wiring alone are 2.3x. Tests are 3.4x.

A sketch in made-up syntax had suggested 4.7x. Writing it in the real grammar, with delivery as a `task` that records each attempt, removed over 40% of that. The causes, judged largest first: imported names are qualified (`orgs.Org`), every field and parameter carries its type, `@uses` and `@why` are written, errors carry their cause and fix text, and the attempt outcome needed its own type.

To reproduce, save the webhooks block from [24](24-examples.md#webhooks-tasks-retries-signing) as `webhooks.slg` and the script below as `measure.py`, then run in a checkout of Ultimate at v31.22.3:

```text
uv run --no-project --with tiktoken python measure.py webhooks.slg
```

```python
import re, sys, tiktoken
enc = tiktoken.get_encoding("o200k_base")
D = "examples/dummy/"; W = D + "apps/web/app/webhooks/"; S = D + "packages/db/src/"
def strip(s):                       # drop // and /* */ outside strings, then blank lines
    o, i = [], 0
    while i < len(s):
        c = s[i]
        if c in "\"'`":
            j = i + 1
            while s[j] != c: j += 2 if s[j] == "\\" else 1
            o.append(s[i:j + 1]); i = j + 1
        elif s.startswith("//", i): i = s.find("\n", i)
        elif s.startswith("/*", i): i = s.index("*/", i) + 2
        else: o.append(c); i += 1
    return "".join(l.rstrip() + "\n" for l in "".join(o).split("\n") if l.strip())
def read(p, n=None): return "".join(open(p).readlines()[:n])
def count(name, parts):
    t = "".join(parts); print(f"{name:8}{t.count(chr(10)):5} lines{len(enc.encode(t)):6} tokens")
src = [read(S + "schema/webhook-endpoints.ts"), read(S + "schema/webhook-deliveries.ts"), read(W + "errors.ts", 40)]
src += [read(W + f) for f in ("entity.ts", "repo.ts", "service.ts", "url-screen.ts", "ledger.ts",
        "actions/add-webhook-endpoint.ts", "actions/remove-webhook-endpoint.ts", "jobs/post-published-webhook.ts")]
tests = [read(W + "endpoints.test.ts"), read(W + "ledger.test.ts"), read(W + "jobs.job.test.ts", 123)]
reg = [D + "apps/web/api/index.ts", D + "apps/web/shared/services.ts", S + "index.ts", S + "client.ts"]
wiring = [l + "\n" for p in reg for l in strip(read(p)).split("\n") if re.search("webhook", l, re.I) and "illing" not in l]
count("source", map(strip, src)); count("wiring", wiring); count("tests", map(strip, tests))
slg = read(sys.argv[1]); cut = slg.index("law cap_holds"); end = slg.index("@target(ultimate)")
count("slg src", [slg[:cut], slg[end:]]); count("slg test", [slg[cut:end]])
```

The two line limits drop a billing receiver that shares the directory.

### What the number hides

| Caveat | Direction |
|---|---|
| The TypeScript carries 2,844 tokens of comments explaining why. The `.slg` carries eight `@why` lines, 148 tokens. Much of that knowledge is not in the spec | favours spec-lang |
| The real job test starts an HTTP receiver and checks a signature over exact bytes. The spec writes `http.calls.first.verifies(a.secret)` and trusts a handler that does not exist | favours spec-lang |
| `orgs.Member`, `posts.Post`, the seed fixtures and `posts.publish` come from a lifted interface module that is not counted | favours spec-lang |
| The spec adds two laws over every interleaving. The TypeScript tests one race | favours TypeScript |

### Where the tokens went

| Layer | TypeScript | spec-lang | Why |
|---|---|---|---|
| Two action declarations | 177 | about the same | Ultimate's `action()` is already this dense |
| Repository, service, ledger adapter, job | 1,980 | the rule and task bodies | No layers. A rule reads and writes entities directly |
| Cap of ten under concurrent adds | a slot column, a unique index, a retry loop | one `@guard` | A rule is atomic, so the guard is the whole statement |
| Wiring | 258 | 0 | Derived from `app` |

The third row is the real finding and the real debt. Postgres does not make a counted guard safe under concurrency. The hand-written code uses ten named slots and a unique index. The backend must now produce that, or a lock, or refuse. See Limits.

## Card form to Ultimate primitive

Paths are in the Ultimate repository.

| spec-lang | Ultimate | Where |
|---|---|---|
| `entity`, `where` claims, `@unique`, `@index` | `entity()`, `invariant()` | `packages/entity/src/entity.ts` |
| `@tenant(org)` | `.tenant()` column | `packages/entity/src/entity.ts` |
| `@secret` field | `.sealed()` | `packages/entity/src/sealed-column.ts` |
| `@access(p else Err)` | a named `can()` policy, shared by every surface | `packages/policy/src/policy.ts` |
| `pub rule` | `action()`, one per file | `packages/action/src/action.ts` |
| `query`, `@live` | `query()`, `live: true` | `packages/query/src/query.ts` |
| `task.start(...)` or `event.emit(...)` inside a rule | an enqueue in the calling action's transaction, one statement for many | `packages/jobs/src/job-enqueue-many.ts` |
| `task` | `job()` with durable steps (next section) | `packages/jobs/src/steps-api.ts` |
| `@on(every(6h))` | Ultimate's `task()`, a cron trigger that enqueues a job | `packages/jobs/src/task.ts` |
| `page` with `@at` | a route and `page.tsx` | `packages/render/src/route.ts` |
| `error` with `@status`, `@fix` | `defineError` | `packages/core` |
| `test`, `law` | the framework's test kinds (below) | `packages/testing` |
| `app` with `@target(ultimate)`, its `http:` and `tools:` lists | registry entries, and `mcp.expose` per action | `apps/web/api/index.ts` in the app |
| a schema change | nothing emitted: the backend runs `x db gen` | `packages/db` |
| module facts | nothing emitted: `x.manifest.json` stays the framework's | `packages/manifest` |

A framework's non-goals become compile errors: `@target(ultimate)` with a non-Postgres store or a React view is refused.

### `task` to durable job steps

The names collide: Ultimate's `task` is a cron trigger. A spec-lang `task` is an Ultimate `job` whose `run` receives a `step` API.

| In a spec-lang `task` | Ultimate | What was read |
|---|---|---|
| a call to a rule | `step.run(name, fn)`: its writes commit alone, and on replay the stored output is returned and `fn` is not called | `StepApi.run` in `steps-api.ts` |
| an outside call whose reply is used | a `step.run` whose call carries an idempotency key built from the run id and the step name. At least once, as the card says | `docs/idea/04-jobs.md` in Ultimate |
| reads between steps | plain reads. Reads are not effects and need no step | same |
| `@idempotent(key)` | `idempotencyKey`, which the job's type already requires | same |
| `@retry`, `@within` | `retry: { attempts, backoff }`, `timeout` | `webhook.ts`, the reference app's job |
| a wait | `step.sleep`, `step.waitForEvent` | `steps-api.ts` |
| a `@secret` value | never a step output: step outputs are stored in a durable table, so the backend re-reads the secret on every attempt | the comment on `endpoint()` in `webhook.ts` |
| "resumes after a crash" | the claim is tested with `crashProof`, which kills the worker at every step boundary and compares effects | `docs/idea/04-jobs.md` in Ultimate |

Two things do not line up. First, a retried Ultimate job replays from the top and skips completed steps, while a spec-lang `@retry` must call out again, so the backend has to put the attempt number into each step name. Second, this task (signed call, attempt ledger, disable after N) is what Ultimate's `webhook()` factory already does. Lowering to plain `job()` re-implements signing and address screening. Emitting `webhook()` is the better output, and it needs a `web` vocabulary kind, not pattern matching on a task body.

## Where Ultimate already wins, and where code goes away

| Area | Verdict |
|---|---|
| Declarations | Ultimate wins already: 177 tokens for two actions |
| Fan-out | Ultimate wins already: route, OpenAPI operation, typed client, job handle, MCP tool and contract test come from one `action()`. Spec-lang derives the `action()`, not these |
| Scaffolding, the gate | Ultimate wins already: `x g resource`, `x verify`, coded errors with `fix:` lines. The backend maps onto them |
| Layers under the declarations, wiring, tests | Code goes away: measured above |
| Screen and error states | Unmeasured. Postly hand-writes a states file and a test file beside each island: 402 lines for one 229-line island |
| One module in several apps | Untested. Ultimate ships no business conventions by design, and a parametrised module is a home for them |
| Web and mobile | Ultimate's own design says every mobile screen is written twice. Spec-lang can derive a screen's data, states, errors and route per target. The view stays a hole or foreign |

## Living inside an existing app

| Question | Answer |
|---|---|
| Where does the source live | `spec/webhooks.slg`, beside the app |
| Where does output go | Ordinary files in the framework's own layout. No bundler plugin |
| Who owns a file | The compiler or a person, never both. Generated files start with `// @generated webhooks#<hash>` |
| Is output committed | Yes, at first. It is never edited, and `spec-lang land` fails on a hash mismatch |
| How does spec-lang call hand-written code | `foreign` |
| How does hand-written code call spec-lang | It imports the generated `action` or service like any other |
| How do I leave | Delete the `.slg` and the headers. Generated code imports only the framework and the app |

Committing output bends "generated software is a build output". The framework's gate and container build read the tree, a reviewer can diff while trust is earned, and leaving costs nothing. Revisit once parity has a record.

A foreign binding is one line, and its evidence starts at `assumed`:

```slg
fn administers(by: Id[orgs.Member], org: Id[orgs.Org]) -> Bool = foreign ts "@postly/core#mayAdministerOrg";
```

Generated code must pass the app's own lint guards with no exemption.

### Lift

| Level | Read from | How | Trust |
|---|---|---|---|
| Interface | `x.manifest.json`, `openapi.json` | deterministic: entities, rule signatures, permissions, jobs, error codes, as declarations with no body | exact |
| Declarations | `entity()` and `action()` object literals | deterministic syntax read | exact for literals, refused otherwise |
| Logic | handler bodies, services, repositories | an agent writes `.slg` | checked, not trusted |

Ultimate already publishes a machine-readable description of every app, so `spec-lang view --from ultimate` can print the first level as modules to `use`. A lifted logic module is accepted when regenerating from it leaves `x.manifest.json` and `openapi.json` byte-identical and the feature's existing tests pass against the generated code. Only then are the hand-written files deleted.

## The same tests, twice

The module runs in the interpreter before any target is chosen, with storage, clock, queue and HTTP as handlers. Then `build` lowers each test to the framework's own kind.

| In the `.slg` | Interpreter | On Ultimate |
|---|---|---|
| scenario with `call -> value;` | in-memory world | `*.test.ts`, calling the action as an actor against embedded Postgres |
| `-> fail Forbidden` by role | guards evaluated | `*.contract.test.ts`, beside the framework's own contract check |
| scenario that drives an `@on` rule | simulated queue, `clock.advance` | `*.job.test.ts` |
| scenario over a `@live` query | replayed change log | `*.live.test.ts` |
| `law` over `Trace[...]` | every interleaving to a bound | a concurrent run against real Postgres: one schedule |

The last row is two strengths of evidence: `bounded(k)` for the meaning, "tested once" for the backend. The report never merges them ([`04-evidence.md`](04-evidence.md)).

## A Tuesday

Task: an org owner may re-enable a disabled endpoint, at most once an hour.

```text
bunx spec-lang view webhooks            # the module, not the eleven files behind it
bunx spec-lang edit webhooks            # add one field, one error, one rule, one test
bunx spec-lang test webhooks            # interpreter: no database, no framework
bunx spec-lang view webhooks --as guarantees --diff     # what a human approves
bunx spec-lang build webhooks           # rewrites generated files, runs x db gen
bunx spec-lang land webhooks            # hashes match, then x verify with the lowered tests
```

The edit adds `enabled_at: Option[Instant] = none` to `Endpoint`, adds `enable` to the app's `http:` list, and:

```slg
@status(429)
@fix("call enable again after {retry_at}")
error EnableTooSoon(id: Id[Endpoint], retry_at: Instant) =
  "endpoint {id} was re-enabled less than an hour ago";

@why("an owner who fixed the receiver should not wait for support, and should not flap it")
@uses(Endpoint.write, clock.read)
@access(administers(by, org) else Forbidden)
@guard(id in Endpoint and Endpoint[id].org == org else NotFound(id))
pub rule enable(by: Id[orgs.Member], org: Id[orgs.Org], id: Id[Endpoint]) -> Id[Endpoint] = {
  match Endpoint[id].enabled_at {
    Some(t) => { if t + 1h > clock.instant() { fail EnableTooSoon(id, retry_at: t + 1h); } }
    none => {}
  }
  Endpoint[id].disabled = none;
  Endpoint[id].enabled_at = Some(clock.instant());
  return id;
}

@given(orgs.seeded)
test enable_is_rate_limited = {
  let e = add(orgs.ada, org: orgs.acme, url: HOOK);
  Endpoint[e.id].disabled = Some("failing");
  enable(orgs.ada, org: orgs.acme, id: e.id) -> e.id;
  enable(orgs.ada, org: orgs.acme, id: e.id) -> fail EnableTooSoon;
  clock.advance(1h);
  enable(orgs.ada, org: orgs.acme, id: e.id) -> e.id;
  enable(orgs.bruno, org: orgs.acme, id: e.id) -> fail Forbidden;
}
```

| Touched by | Files |
|---|---|
| The agent | `spec/webhooks.slg` |
| The compiler | the endpoint schema file, `errors.ts`, a new `actions/enable-webhook-endpoint.ts`, the service, the test file, the registry barrel |
| Ultimate | the migration, `x.manifest.json`, `openapi.json` |
| A person | the claims diff: one new rule, who may call it, one new error |

This is a target. The saving against doing the task by hand is not measured.

## Other frameworks

A third party writes the contract in [`11-backends-and-parity.md`](11-backends-and-parity.md). The prediction to test: a backend grows with the guarantees its framework lacks, not with its API. Ultimate lacks atomic rules under concurrency. Rails also lacks one authorisation path and tenant scoping. Next lacks nearly everything. Terraform has no logic to carry.

Choose per module. Go below a framework when there is no framework process, when a `@within` or `@alloc` budget cannot be met, or when about half the module would be `foreign`. `spec-lang` and one backend package per framework major are development dependencies, so `bunx spec-lang` works and nothing generated imports them.

## Limits

- **The measured win is 2.7x and mostly not a framework win.** It comes from removing layers and wiring. Declarations and fan-out are where Ultimate was already dense.
- **Atomic rules on Postgres are designed, not shown.** A guard that reads a count is safe only with a constraint, a conditional counter, a lock or a serialisable retry; [11](11-backends-and-parity.md) gives the choice and when to refuse. No line of it has run against Postgres.
- **Four things the feature needed beyond the card** are vocabulary, specified in [`29-first-vocabularies.md`](29-first-vocabularies.md): an `error` kind with status, cause and fix; returning a `@secret` once (`@reveals`); the attempt number inside a task (`run.attempt`); and running started tasks to completion in a scenario (`tasks.drain()`).
- **Retry against replay.** See the task section: attempt-numbered step names are a design, not a tested answer.
- **Registries.** Ultimate apps register features in hand-maintained files. Either the app adds one import per registry, or Ultimate accepts a fragment.
- **Lifted logic is only as faithful as the tests that accept it.** No screen was measured.
