# spec-lang

A programming language for AI authors. **The AI writes logic and tests, the interpreter runs them at once, the compiler generates the software.** Existing languages and frameworks are the compiler's intermediate representation.

**Status `As of 2026-11-24`:** design. Only the grammar runs. Never describe anything else as existing.

## Response rules
Execute, no preamble. Read before speculating. Disagree when wrong. Terse. End summary: 2 sentences at most.

## Rules
| Rule | Means |
|---|---|
| The card is the charter | `docs/idea/01-language-card.md` wins; change it first, then the docs that use it |
| Every `slg` fence must parse | grammar in `docs/idea/25-grammar.md`; not source means another fence language |
| Grammar never grows per domain | a domain adds kinds and notes; a new form needs a decision record |
| Admission test | easier for a model to express or to check, and fits an existing form; else library, else rejected with the reason |
| Minimal and sufficient | two possible behaviours is a question, never a guess |
| Honesty | no invented number; a measured one carries method and caveat; every doc ends with Limits |
| Private stays private | no private repo, client, person, host or local path in any file; corpus cases stay generic |
| One path | no shims, no parallel syntaxes; replace completely |
| No worktrees | big work: a team in this checkout, disjoint file sets |

## Map
| Path | Holds |
|---|---|
| `docs/idea/` | the design; index in `docs/idea/README.md` |
| `docs/decisions/` | one record per one-way door |

Markdown only: no code, no build, no CI. Check links and `slg` blocks before committing.

Conventions: the `gold-standards-in-ai` repo.
