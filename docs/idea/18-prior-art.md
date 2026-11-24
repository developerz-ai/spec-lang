# Prior art

> **Design.** Nothing here runs yet. Who already built parts of this, what happened to them, and what the evidence says about models writing code.

Checked 2026-11-24. Every linked source was opened, most through a summarising reader; papers are mostly preprints. "Vendor" means a project measured itself. What was not opened is not here.

## Verdict

A language for AI authors with contracts, effects and a solver is **not new**. One source with many backends is **not new** and has a poor record. Two facts weigh most against us: the nearest sibling retired its own language, and models pay for unfamiliar ones. What may be new is a combination, listed last.

## Languages and tools for AI authors

| Project | What it does | State seen | Lesson |
|---|---|---|---|
| [Djinnlang](https://arxiv.org/pdf/2609.23954) | programs are only specs; **an LLM inside the compiler** writes code and proofs, Dafny checks; a spec must also be *unambiguous*, so any two valid outputs are equal | self-hosted: 4,088 spec lines, 31,823 proof lines, 1.4B tokens, days. No concurrency; thin effects | the closest idea. **We put no model in the build**: fills are committed source and builds are repeatable. We take the unambiguity test for `declare` bodies |
| [Vera](https://raw.githubusercontent.com/aallan/vera/main/README.md) | mandatory contracts and effects, Z3, runtime fallback, unguarded sites disclosed; no variable names | working, pre-1.0 | nearest to our evidence ladder. We keep names |
| [Aver](https://docs.rs/crate/aver-lang/0.24.1) | cases and laws beside functions, effects in signatures, `decision` blocks, budgeted context export; Lean and Dafny export | working | `why` in source, tests beside logic and context slices are **borrowed ground** |
| [MoonBit](https://docs.moonbitlang.com/en/latest/language/verification.html) | general language; `moon prove` through Why3 | proof support experimental | proofs per package, assumptions listed |
| [Bosque](https://arxiv.org/abs/2407.06356) | remove features that make reasoning hard | research | "regular, not easy" predates us |
| [Allium](https://github.com/juxt/allium) | behaviour specs beside code; distils specs from code; no compiler | active | distilling is a cheap way into old code |
| [Kiro](https://kiro.dev/blog/deep-spec-analysis/) | an LLM formalises requirements, a solver reports conflicts as two-option questions | shipping | ask a choice, never show a formula |
| [CodeSpeak](https://codespeak.dev/) | programs in structured human language, enforced through agents | waitlist; calls itself an agentic engineering toolkit | the spec stays prose and an agent implements it |
| [Tessl](https://codemyspec.com/blog/tessl-review) | spec as source | one reviewer: repositioned to agent skills, Jan 2026 | a [field review](https://www.martinfowler.com/articles/exploring-gen-ai/sdd-3-tools.html) saw non-determinism and asked if this joins MDD's inflexibility to it |
| [Verse](https://verselang.github.io/book/13_effects) | effect specifiers in signatures; failure undoes effects | documented by its maker | effects in signatures are borrowed |
| [SpecLang](https://githubnext.com/projects/speclang/) | prose code compiled by an LLM, 2023 | research prototype | near-identical name; expect confusion |

## Custom languages and many backends

| Case | Outcome | Lesson |
|---|---|---|
| [Wasp](https://wasp.sh/blog/2026/05/13/new-language-for-web-dev-was-a-mistake) | after five years called its language a mistake; editor tooling reached 80% of its goal; users wrote 90% React and Node anyway. [Retired in 0.24](https://wasp.sh/docs/guides/legacy/wasp-dsl) for a TypeScript spec. The whole-app understanding stayed | the semantics were the value, the syntax the cost. Our reply, that the author is a model, must be **measured** |
| [Wing](https://www.calcalistech.com/ctechnews/article/bj90wnmrjl) | company closed; interest did not become a business | developer experience alone does not pay |
| [Darklang](https://blog.darklang.com/goodbye-dark-inc-welcome-darklang-inc/) | structured editor "didn't make sense when the LLM is generating the code" | plain text is the source; projections are views |
| [MDE, 450 surveyed](https://staffwww.dcs.shef.ac.uk/people/A.Simons/remodel/papers/WhittleMDE_Draft.pdf) | typical gain 20 to 30%, easily offset; narrow domains work; most fail at scale-up; one case of unreadable output raised certification cost eightfold | agents remove training cost, not scale-up or output quality |
| [Shared C++ on mobile](https://dropbox.tech/mobile/the-not-so-hidden-cost-of-sharing-code-between-ios-and-android) | dropped: own frameworks, own tooling, platform differences | lower to each platform's own framework |
| [MLIR](https://www.modular.com/blog/democratizing-ai-compute-part-8-what-about-the-mlir-compiler-infrastructure) | infrastructure adopted, dialects fragmented; "scaling too early" | small curated vocabulary set first |
| [Dafny](https://dafny.org/) | verified code compiled to C#, Java, JavaScript, Go and Python; the nearest relative on proofs plus backends | proofs and many backends do fit together; it stops at code, not schemas, UI or infrastructure |
| [Haxe](https://haxe.org/documentation/introduction/compiler-targets.html) | one language, many targets, graded in tiers; some targets were later removed | publish a tier per backend, and retire one openly |
| [Nim](https://nim-lang.org/docs/backends.html) | C, C++, Objective-C and JavaScript, calls in both directions | bidirectional interop makes it usable inside existing projects |
| [Vala](https://vala.dev/) | native language with first-class GObject and GTK support | a language can be shaped around one framework; its reach is that framework's |
| [Halide](https://halide-lang.org/) | works: embedded, algorithm apart from schedule | logic apart from layout, in a narrow domain |
| [Unison](https://www.unison-lang.org/unison-1-0/) | language and runtime reached 1.0 | we take content hashing, as a cache key only |

## Proofs, systems, simulation

| Evidence | Finding | Lesson |
|---|---|---|
| [Vericoding](https://arxiv.org/abs/2509.22908) | spec given: 82% Dafny, 44% Verus, 27% Lean | proving a given claim often works |
| [Vero](https://rdi.berkeley.edu/blog/vero) | whole Lean repositories: best 63%, next 19%; best passes 87% of single specs | success compounds badly |
| [VeruSAGE](https://arxiv.org/html/2512.18436v2) | 81% of 849 system proof tasks, specs given; one agent cheated in 14% without a cheat checker | check the checker |
| [OSS-Fuzz proofs](https://www.anthropic.com/research/formal-verification) | crash-freedom for 239 entry points; about 180M proof lines; hundreds of millions of tokens each | possible, not cheap |
| [VerusEval](https://arxiv.org/abs/2610.26464) | 42.3% of verifier-accepted specs pass all semantic checks | **the spec is the weak link** |
| [Beyond `assume false`](https://neurips.cc/virtual/2026/171449) | up to 39.1% of DafnyBench tasks gamed; hardening costs at most 6.1% | [anti-gaming](08-approval-and-anti-cheating.md) is core work |
| [AI-written specs](https://buttondown.com/hillelwayne/archive/llms-are-bad-at-vibing-specifications) | some properties are tautologies | mutation-test every claim |
| [Mariposa](https://www.andrew.cmu.edu/user/bparno/papers/mariposa.pdf) | 2.6% of 17,043 SMT queries unstable | pin solvers, cache certificates |
| [Verified distributed systems](https://blog.acolyer.org/2017/05/29/an-empirical-study-on-the-correctness-of-formally-verified-distributed-systems/) | 16 bugs, none in protocols, 11 in the shim | bugs live at the boundary: grade `foreign` |
| [Verified zlib, fuzzed](https://kirancodes.me/posts/log-who-watches-the-watchers.html) | memory bug in the runtime, none in verified code | name what is trusted |
| [seL4](https://trustworthy.systems/projects/OLD/l4.verified/numbers) | about 25 person-years in total, research and tools included | `machine` altitude is the costly one |
| [TigerBeetle](https://tigerbeetle.com/blog/2025-06-06-fuzzer-blind-spots-meet-jepsen/) | a strong simulator missed a query bug: its workload never made the input | simulation is as good as its generator |
| [S3 ShardStore](https://aws.amazon.com/blogs/storage/how-automated-reasoning-helps-us-innovate-at-s3-scale/) | property tests and model checking over the real Rust code | several kinds of evidence at once is normal practice |

I found no OS or database written by agents and verified end to end.

## Models and languages

| Question | Evidence | Reading |
|---|---|---|
| Does a strict compiler help? | Idris: [22 of 56 to 54 of 56](https://arxiv.org/abs/2602.11481) with compiler errors in the loop | best single result for our loop |
| Does error detail matter? | [41.7% to 52.8%](https://arxiv.org/html/2606.01522v1) on a small model; near ceiling on a stronger one, where a bare error lost to no types once names were obfuscated | a terse error can be worse than none |
| Do types constrain usefully? | type-constrained decoding [halves compile errors](https://arxiv.org/abs/2504.09246) | needs decoder access; assume we lack it |
| Can models use an unseen language? | Vera [98.7%](https://raw.githubusercontent.com/aallan/vera-bench/main/README.md), Python 96.7%, TypeScript 99.7% (vendor, 60 tasks, saturated); [Anka](https://arxiv.org/html/2512.23214v1) 95.8% against Python 91.2% (own benchmark, one domain) | at function scale, yes |
| When does it fail? | Gleam and MoonBit [about 1%](https://arxiv.org/html/2606.16827) zero-shot on a hard set; [0 to 11%](https://arxiv.org/abs/2603.09678) on esoteric languages | novelty costs; strong agents [write generators in Python instead](https://arxiv.org/abs/2606.10933) |
| What predicts DSL failure? | [18 DSLs](https://arxiv.org/abs/2611.05168): 77% of variance is the DSL, 8% the model; knowledge demand is the top factor | every construct is paid for |
| What costs tokens? | OCaml [1.28 to 1.69 times](https://arxiv.org/pdf/2607.22807) Python, mostly from revising, not length | density is not the headline metric |
| Does surface density help? | [10 to 14%](https://arxiv.org/abs/2404.16333) from a model-oriented grammar; formatting removal barely moves [output](https://arxiv.org/html/2508.13666v1); a compact data format [lost about 9 points](https://arxiv.org/html/2605.29676) of accuracy | densify meaning, not spelling |
| Do names matter? | removing them cost [11 to 29 points](https://arxiv.org/html/2510.03178v1) of class summaries | keep names |
| Typed or dynamic? | [one benchmark](https://dev.to/mame/which-programming-language-is-best-for-claude-code-508a) favours dynamic; [a re-check](https://danluu.com/pl-tokens/) found its Rust failures were a harness bug and no strong effect | unsettled; do not cite |
| DSL or host language? | AMPL [competitive with Python](https://arxiv.org/abs/2601.00469) | a DSL need not lose |

I found no controlled study of indentation against braces for models. [19](19-density-and-experiments.md) must run its own.

## Not new: do not claim

Contracts and effects for AI code, solver counterexamples, errors written for agents, builds without a model, intent blocks, token-budgeted context, refinement and unit types, effects in signatures, content hashing, deterministic simulation, scenarios, many-target code generation.

## Possibly new

Found nobody doing it. That is weak evidence: the search was days, not a survey.

1. **Evidence kept through frameworks.** Each claim is re-graded per backend ([11](11-backends-and-parity.md), [12](12-frameworks.md)). Vera and Aver grade; Wasp lowers to frameworks; none found does both.
2. **Weakest-link evidence across altitudes.** A `foreign` body visibly caps its callers' grade, in the diff ([04](04-evidence.md)).
3. **One counterexample, one transaction.** A failing law returns state, call, and edits covering the source and everything derived from it ([07](07-agent-loop.md)).
4. **Grammar admitted by experiment.** Each form kept or dropped on published measurements across model families.
5. **Projections graded by planted faults.** A view for humans carries the rate at which reviewers catch faults planted in the source.

Item 3 needs derived artefacts that do not exist yet. Items 4 and 5 are only promises until run.

## Limits

- No independent measurement of any AI-first language at repository scale was found. Our main claim has no precedent to lean on.
- Benchmarks age in months. Model names are omitted on purpose; re-check before reuse.
- Several papers disagree with their own tables (the compact-format, spec-quality and names papers). Ranges here follow tables or abstracts as read.
- Not opened: F*, Liquid Haskell, Alloy, P, CertiKOS, IronFleet's own paper, FSCQ, Redox, Racket's literature, CASE and 4GL studies.
