# Thesis

> **Design.** Nothing here runs yet. Why the language exists, what it locks down, what would prove it wrong.

**Executable pseudocode for AIs.** The AI writes logic and tests. The interpreter runs them at once. The compiler generates the software.

Pseudocode is how models and people already think about a program: no plumbing, only what happens. Its two faults are that it is vague and that it does not run. spec-lang keeps the form and removes the faults: every program has exactly one meaning, and it is tested the moment it is written.

Every program is logic: something is true, something happens, something else becomes true. The rest is plumbing between that logic and a machine. In a corpus of ten private repositories, one opinion per person per item (set, clear, list) took about 2k lines in about 50 files, and one reader judged 10 to 20% of its production code essential ([16-corpus](16-corpus.md)). spec-lang is a programming language in which the plumbing is not written.

**The aim is a shorter dev cycle.** Define logic fast, test it at once, find out early whose fault a failure is. Less code, cheaper review and safer software follow from that, and where they conflict with it the cycle wins, except for safety.

## Who it is for

| Reader | Needs | Consequence |
|---|---|---|
| **The author: a model** | little context, one way per thing, instant truth, no guessing | dense regular source, tests beside logic, errors with the failing input and the edit |
| **The approver: a human** | to know what changed in meaning | compiler-rendered projections and a guarantee diff; source may be hard for humans ([08](08-approval-and-anti-cheating.md)) |

The language is made by its user. Ties are settled by what helps a model write it, and model opinion counts as a hypothesis until a trial measures it ([02-syntax](02-syntax.md)).

## The axioms

| # | Axiom | Means |
|---|---|---|
| 1 | **Logic first** | a module runs and tests in the interpreter before a target, framework or database is chosen; holes run too |
| 2 | **It is a programming language** | step-by-step code is always available; every declaration is shorthand for an ordinary program you can view |
| 3 | **One core form** | state + event -> new state + effects + reply, from a tap to a syscall |
| 4 | **Minimal and sufficient** | drop a word only if one visible rule recovers it; two possible behaviours is a question, never a guess |
| 5 | **Languages are intermediate** | TypeScript, Rust, C++, Swift, SQL, and frameworks above them, are build output; nobody edits it |
| 6 | **No model in the build** | models write source; compilation is deterministic, hermetic, replayable |
| 7 | **Highest target that keeps the guarantee** | lower to a framework when it preserves what was declared; refuse when nothing can |
| 8 | **Always a way down** | declare, define, machine, foreign: one type system, so no wall at the edge of an abstraction |
| 9 | **Evidence is never rounded up** | every claim says how it is known and what it trusts |
| 10 | **Small core, open vocabularies** | a domain adds kinds and notes in one fixed shape; the grammar never grows |

## What is written, what is derived

| Written | Derived |
|---|---|
| types and what they mean | schema, migrations, parsing, validation, wire formats |
| rules, flows, algorithms | transactions, races, retries, error mapping, jobs |
| who may do what | checks on every surface |
| examples, properties, laws | suites for the interpreter and every backend, monitors |
| what a screen or API shows | routes, clients, pagination, caching, loading and error states |
| budgets and deadlines | layout choices, alarms, deploy configuration |

Each derived item is a named policy with a default the author can override. Anything else would be a cage the first time a default is wrong.

## What it is not

- **Not shorter spelling.** One published model-oriented grammar saved [10 to 14%](https://arxiv.org/abs/2404.16333) of tokens, and our own count of one program shows symbols costing tokens, not saving them. The gain must come from meaning ([18-prior-art](18-prior-art.md), [02-syntax](02-syntax.md)).
- **Not a prompt format.** A program has exactly one behaviour, or the compiler reports the open choice.
- **Not a configuration language.** Those end at the wall their authors built. Wasp's web language, the nearest relative, was [retired for TypeScript](https://wasp.sh/blog/2026/05/13/new-language-for-web-dev-was-a-mistake) in 2026 ([18-prior-art](18-prior-art.md)).
- **Not equally dense everywhere.** Kernels, codecs and simulations are already mostly logic. There the gain is checking, not size.

## What would prove it wrong

The same semantics embedded in TypeScript, with the same server and tests, is the control. If spec-lang does not beat it on tokens and escaped defects per accepted change, we ship the embedded form and drop the bespoke syntax. The experiment runs **before** the compiler is built ([19-density-and-experiments](19-density-and-experiments.md)).

## Limits

- Specification independence: if one model writes the logic, the laws and the tests, all three can miss the same idea. Tooling exposes ambiguity; it cannot invent a requirement nobody asked for.
- "Proven" is proven of the meaning. A framework's fidelity to it is tested, not proven ([04-evidence](04-evidence.md)).
- No training corpus exists. The card, the errors and the trials have to carry that.
