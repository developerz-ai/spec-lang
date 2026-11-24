# Syntax

> **Design.** Nothing here runs yet except the parser and the token counts below. Why the surface looks as it does, what was measured, and what would change it.

The [card](01-language-card.md) is the syntax. This file is the argument for it. Preference order: one way per thing, local edits, loud errors, then tokens.

## Decisions

| Question | Decision | Why |
|---|---|---|
| Declaration forms | one: notes, kind, name, data, result, body | domains add words, never grammar |
| Data and policy | `( )` holds invented names, `@note` holds closed ones | a field called `access` cannot collide with the policy `@access` |
| Notes | prefix, typed, one meaning; order matters only among `@guard`s, which run top to bottom | attach to parameters and declarations alike. An earlier `name: value;` slot form could not mark a parameter |
| Altitude | the form of the body | the interface is everything left of `=`, so callers do not change |
| Blocks | braces; indentation carries no meaning | a lost indent changes nothing, a lost brace is an error (below) |
| Statement end | `;` or a closing `}`; no newline rules | costs nothing on seven tokenizers, about 1% on two |
| Type arguments | `List[Int]` | `<` is only comparison; types and expressions share one grammar |
| Ranges | `..=` and `..<`; bare `..` illegal | the off-by-one is spelled out |
| `%` | percent only; remainder is `mod` | one meaning |
| Lambda | `it`, or `x => ...` when nested | 4 to 2 or 5 to 3 tokens, by one visible rule |
| Block value | explicit `return` | no tail expression that a stray `;` deletes |
| Flow moves | one repeated `@edge(A -> B, via:, by:, after:)` per move | an edit touches one line; a list of arrows had no room for who and when |
| Waiting | `rule` and `task` are kinds, not syntax | the same shape; the reader sees atomic or resumable in the first word |
| Inference | inside a module; `pub` states types and effects | an edit cannot change the meaning of a distant file |
| Overloads, shadowing, wildcard imports, user operators | none | each makes a name depend on somewhere else |

## Grammar facts

[The grammar](25-grammar.md) builds in Lark's strict mode as LALR(1) with zero conflicts and a lexer that needs no parser state. 30 reserved words, 41 nonterminals. Strict mode finds conflicts, not wrong trees: it missed that `x²` lexed as one name (fixed).

The same file can drive constrained decoding. Mask with the complete base grammar, never with a vocabulary's closed list unless that list is complete: [a 2026 preprint](https://arxiv.org/abs/2606.21619) reports that constrainers rejecting valid programs cut functional correctness by up to 97% (read in summary only).

## Token counts

**Method.** Nine tokenizers: tiktoken `o200k_base` and `cl100k_base`, the tokenizers of Qwen3, DeepSeek-V3, Llama 3.2, Gemma 3, Mistral Nemo and GLM-4.5, and Anthropic's published Claude 2 tokenizer, which is obsolete. **No current Claude tokenizer was measured.** Tokens are a cost, not a measure of whether a model reads or writes a form correctly.

### Symbols

Extra tokens for the symbol inside `x _ y`, or whole-expression counts where shown; smallest to largest over the nine.

| Form | Symbol | ASCII | Word | Canonical |
|---|---|---|---|---|
| `≤ ≥ → ×` | 1 | 1 | | ASCII (tie) |
| `≠` | 1 to 2 | 1 | | ASCII |
| `∧ ∨ ¬` | 1 to 2 | 1 | 1 | word |
| `∈ ∀` | 1 to 2 | | 1 | word |
| `⇒` | 1 to 3 | 1 to 2 | 1 | word |
| `∃ ∑ ∫ ∂ √ ∪ ∩ ∅ ∞` | 1 to 2 | | 1 | word |
| `↦ ⊥ ⊆` | 1 to 3 | | 1 (`↦`: ASCII `=>`) | word or ASCII |
| `a ÷ b`, `a / b` | 3 to 4 | 3 | | ASCII |
| `∀ x ∈ s: p(x)`, `s.all(p)` | 8 to 11 | | 4 to 6 | call |
| `∑ᵢ xᵢ·yᵢ`, `sum(x * y)` | 7 to 14 | | 5 to 6 | call |
| `5µs`, `5us` | 3 to 4 | 2 | | ASCII |
| `x²`, `x^2` (same for `³`) | 2 to 3 | 3 | | symbol |
| `80°C`, `80degC` | 2 to 4 | 3 to 4 | | symbol |
| `x ± y`, `near(x, y)` | 3 | | 5 to 6 | symbol |

**Rule.** A symbol becomes canonical only if no tokenizer charges more for it and at least one charges less. The rest still parse where they map one-to-one, and the formatter prints ASCII. On the example programs both spellings lex to the same stream, text literals aside. Decision record: [004](../decisions/004-ascii-canonical-unicode-accepted.md).

### One program, three notations

[the seats module](24-examples.md#seats-pure-logic-scenarios-laws), an idiomatic Ruby + minitest equivalent that runs green, and a Unicode rewrite with set-builder and quantifiers that is not valid spec-lang. The last two are not in this repo.

| | chars | o200k | cl100k | Qwen | DeepSeek | Llama | Gemma | Mistral | GLM | Claude 2 |
|---|---|---|---|---|---|---|---|---|---|---|
| Ruby | 2937 | 947 | 952 | 1009 | 1000 | 952 | 1105 | 1059 | 952 | 962 |
| spec-lang | 1638 | 578 | 580 | 606 | 605 | 580 | 690 | 621 | 580 | 653 |
| maths | 1561 | 594 | 584 | 610 | 604 | 584 | 686 | 630 | 584 | 665 |

spec-lang is 32 to 41% below Ruby. Without tests and contracts the gap is 16 to 25%. Maths saves 5% of the characters and costs more tokens on seven of nine.

**Caveats.** One program, chosen and written on both sides by the language's designer. A library could shorten Ruby's generators and trace loop. Ruby needs no card: ours is 4,884 o200k tokens on 2026-11-24, so at 369 saved per module of this size it takes fourteen modules to break even, before prompt caching and any repair. The [library reference](28-standard-library.md), if loaded whole, is another 5,000. [19](19-density-and-experiments.md) owns the real comparison.

### Layout

Text transforms of the eight programs in [24](24-examples.md) on 2026-11-24 (5,073 o200k tokens), change against the canonical form, range over nine.

| Variant | Change |
|---|---|
| Unicode operators | 0.0 to +0.9% |
| no line-end `;` | -1.1 to 0.0% |
| offside (no block braces, no `;`) | -2.8 to -1.2% |
| one-space indent | -3.4 to 0.0% (0.0% on Claude 2) |
| tab indent | -2.1 to +3.0% |

None pays for a less familiar form whose edit accuracy is unmeasured. For scale, [Pan et al.](https://arxiv.org/abs/2508.13666) stripped formatting from existing languages: 24.5% fewer input tokens on average, worst Pass@1 drop 4.2%, on fill-in-the-middle tasks.

## Fault probes

Run against the current grammar and examples. Synthetic faults, not model mistakes.

| Probe | Result |
|---|---|
| delete each `}` in code | 50 of 50 fail to parse; reparsing from each declaration at column 0 pins all 50 to the right declaration |
| delete each `}` inside text | 0 of 9 fail: interpolation is opaque to the grammar |
| remove one line's indentation | 182 of 182 give the same tree |
| 20 habits from mainstream languages | 18 fail to parse |

The 18 include `List<Int>`, `a % 2`, `&&`, `? :`, `+=`, `0..10`, a tail expression, `-> T { ... }` without `=`, `enum A { }` and `entity T { title: Text; }`. The two that parse are legal text: `const A = 3;` and an ill-typed lambda. The dangerous class parses and means something else; 20 hand-picked probes cannot show it is empty.

Writing the first seven programs, the designer made six syntax errors: three grammar bugs (fixed), an undefined operator, and two reserved words used as names. Expect models to repeat the last kind.

## Blind trial

**Method.** An author sees the card and nothing else: no parser, no other doc. It writes ten programs: ratings, invitations, retention, ledger, elevator, rate limiter, ring buffer, train, text diff, webhook retry. It marks each guess with a comment. The grammar runs once, afterwards.

**First author (Opus).** 9 of 10 files parsed first try. The failure was a bare `return;`, which the grammar then gained, making the same files 10 of 10. The first number is the result. The second only says the fix works: those files chose it.

The same ten files carry 183 lines marked as guesses. Parsing was the easy half; what the card left unsaid was meaning, and its Data and "How it runs" sections were added in answer.

**Second author (Fable), same card and tasks.** 7 of 10 first try. Failures: a chained `else if ... then`, which the grammar then gained; `20µs`, now an accepted spelling of `20us`; and a statement used directly as a match arm, which stays an error and is now on the card. Together the first-try score is 16 of 20 files.

Independently, both asked for the same five things: records and collections, time and number rules, trace contents, effects and their failures, and one worked line per form named. Both judged retention and text diff easier in SQL or Python.

**Third author (Haiku, a small model).** 10 of 10 first try, with 35 guess lines, but on a later card that holds fragments close to most of the tasks, so the score does not compare with the first two. It says a small model can write the form. It does not say the logic is right: its own notes admit an elevator that can starve a floor, and it found that nothing ran when retries were exhausted, which the card now covers.

**What it cannot show.** Nothing type-checked or ran. One model family so far, the one that wrote the card. The card has since gained fragments close to seven of the ten tasks (ratings, invitations, retention, ledger, elevator, ring buffer, webhook), so later authors on those tasks are not blind to them, and their score cannot be compared with the first. New tasks are needed.

## Experiments that would overturn choices

Thresholds are proposals, to be fixed before each run.

| Choice | Experiment | Overturned when |
|---|---|---|
| A new language at all | total tokens and turns to green, card included, against TypeScript on Ultimate, three model families | [19](19-density-and-experiments.md) sets the threshold; [001](../decisions/001-own-surface-syntax.md) names the fallback |
| Notes as open words in the grammar | unknown or misplaced notes per 100 declarations | above 3 after one repair |
| Braces | corrupt one line in braces and in an offside twin; count silent changes of meaning and repair turns | offside is no worse on both |
| ASCII canonical | same tasks with either spelling required; score correctness | Unicode is more correct |
| Two spaces | exact-edit success with one and two spaces | one space is no worse |
| `->` for result, function, edge, table row and step | authors and readers with `yields` in tests | fewer wrong readings with separate words |
| `x[m]` as index or unit | misread rate on mutated programs | above 1%; then `unit"m/s"` |
| Notes before the names they mention | edit tasks; count contracts left stale | suffix notes go stale less |
| `law` apart from `test` | models label 100 claims independently | agreement below 80%; then merge |
| `it` | mutate nesting; count captures of the wrong argument | any silent capture |

## Limits

- **Regular is not learnable.** Zero conflicts says the text is unambiguous. It says nothing about whether models write correct programs in a language with no training data.
- **Uniformity moves errors.** One shape means an invented `@retry_count(3)` parses. The vocabulary checker, which does not exist, must catch it.
- **Text is opaque.** A broken `{name}` inside a literal parses; the checker must look inside.
- **Small samples everywhere.** Eight programs, one Ruby comparison, synthetic faults. Examples and card still change: counts are dated, not live.
