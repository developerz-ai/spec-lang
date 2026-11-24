# The idea

Read [`00-thesis.md`](00-thesis.md), then [`01-language-card.md`](01-language-card.md). Everything else expands one axiom.

> **Design.** `As of 2026-11-24` only the grammar runs: every `slg` block here parses. There is no interpreter and no compiler.

| Doc | Hook |
|---|---|
| [`00-thesis.md`](00-thesis.md) | The AI writes logic and tests, the interpreter runs them, the compiler generates the software. Ten axioms, and what would prove it wrong. |
| [`01-language-card.md`](01-language-card.md) | The whole language on one page. What a model gets before it writes. The charter: it wins over every other doc. |
| [`02-syntax.md`](02-syntax.md) | Why it looks like this: token counts on nine tokenizers, fault probes, blind-author trials. |
| [`03-types-and-claims.md`](03-types-and-claims.md) | A type is a kind of value plus claims. Units, money, states and budgets from one mechanism. |
| [`04-evidence.md`](04-evidence.md) | How each claim is known, what it trusts, how that survives calls, foreign code and backends. |
| [`05-effects-and-trust.md`](05-effects-and-trust.md) | Effects, capabilities, secrets, tenants, and model output as an untrusted type. |
| [`06-logic-and-tests.md`](06-logic-and-tests.md) | The centrepiece: idea, logic, test, counterexample, fix. Scenarios without step definitions. |
| [`07-agent-loop.md`](07-agent-loop.md) | Six verbs, diagnostics that carry the edit, token-budgeted slices, many agents on one checkout. |
| [`08-approval-and-anti-cheating.md`](08-approval-and-anti-cheating.md) | What a human approves without reading source, and how an agent is kept from weakening a law to go green. |
| [`09-vocabularies.md`](09-vocabularies.md) | What a domain module is: kinds, notes, laws, expansion. Terminating, hygienic, composable. |
| [`10-compiler-and-ir.md`](10-compiler-and-ir.md) | Interpreter first, then three semantic layers down to target trees. Deterministic planner. |
| [`11-backends-and-parity.md`](11-backends-and-parity.md) | A backend claims, proves parity by running the same tests, or refuses. |
| [`12-frameworks.md`](12-frameworks.md) | Ultimate as the first backend, one feature measured, an ordinary Tuesday. |
| [`13-games.md`](13-games.md) | Deterministic simulation logic over an in-house C++ sim today and anygame later. |
| [`14-systems.md`](14-systems.md) | A kernel, a database, and the compiler in itself: what is gained, what stays by hand. |
| [`15-domain-tour.md`](15-domain-tour.md) | One short program per domain, web to robotics. |
| [`16-corpus.md`](16-corpus.md) | Where small ideas became large code, counted, and eight cases where a language would not help. |
| [`17-inspirations.md`](17-inspirations.md) | Ideas taken from languages, shells, maths and the sciences, and what was left behind. |
| [`18-prior-art.md`](18-prior-art.md) | Who tried, what happened, and the parts nobody seems to be doing. |
| [`19-density-and-experiments.md`](19-density-and-experiments.md) | The metric, the control arms and the kill criteria. |
| [`20-roadmap.md`](20-roadmap.md) | Cheapest disproof first. |
| [`21-risks.md`](21-risks.md) | Twelve ways this dies, each with a trip-wire. |
| [`22-lifecycle.md`](22-lifecycle.md) | Schema evolution, migration, production feedback, adopting it in an existing app. |
| [`23-memory-concurrency-distribution.md`](23-memory-concurrency-distribution.md) | Immutable by default, compile time for free, no async colouring, parallel by purity, places and replicas in the types. |
| [`24-examples.md`](24-examples.md) | Whole programs, all designer-written, all parsing. |
| [`25-grammar.md`](25-grammar.md) | The grammar: the one part that has been run. |
| [`26-program-structure.md`](26-program-structure.md) | How to lay out a program, from one file to a fleet: modules, ownership, boundaries, apps. |
| [`27-authorization.md`](27-authorization.md) | One model for who may do what, on every surface. |
| [`28-standard-library.md`](28-standard-library.md) | Every core name with its signature. |
| [`29-first-vocabularies.md`](29-first-vocabularies.md) | The exact kinds and notes of `web`, `jobs`, `errors`, `game` and `storage`. |
| [`30-type-rules.md`](30-type-rules.md) | Typing, effects and evaluation, precise enough to build one checker twice. |
| [`31-dropping-down.md`](31-dropping-down.md) | A little backend code behind a contract, as C keeps a little assembly. |
| [`32-build-layout.md`](32-build-layout.md) | Where generated software goes, compared with TypeScript. |

## Reading paths

| You are | Read |
|---|---|
| Deciding if this is worth it | `00`, `16`, `18`, `19`, `21` |
| A model about to write it | `01`, then [24](24-examples.md) |
| Building the interpreter | `01`, `06`, `03`, `10` |
| Building a backend | `11`, `12`, `09` |
| Using it in an Ultimate app | `12`, `22`, `07` |
| Here for the OS and database | `14`, `03`, `04` |
