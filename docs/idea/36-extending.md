# Extending the language

> **Design.** Nothing here runs yet. What replaces macros, metaprogramming and code generators, and what is left out on purpose.

**You can add words, meanings, worlds and targets. You cannot add grammar.** A model that knows the [card](01-language-card.md) can read any project, because no project has invented syntax.

## Five places to extend

| Add | With | Example |
|---|---|---|
| a word that declares | `kind` | `entity`, `service`, `system` |
| a policy or claim | `note` | `@page`, `@retain` |
| a world | `handler` | a simulated payment provider |
| a way to implement | a strategy in a backend | keyset pagination |
| a target | a backend | Ultimate, Rails, WGSL |

Each is an ordinary declaration with a type, tests and laws ([09](09-vocabularies.md), [11](11-backends-and-parity.md)). The first three travel together as a library, `libs/<name>/`: kinds and notes in `vocab.slg`, the calls it offers in `contract.slg`, its worlds in `pretend.slg` ([card, Libraries](01-language-card.md#libraries)). The compiler loads a library and knows none by name, so a new domain is a new directory, never a compiler change.

## What people use macros for

| Use | In other languages | Here |
|---|---|---|
| boilerplate | derive macros, decorators | a `kind` expands to ordinary declarations |
| retries, caching, logging around a call | wrappers, aspects | a `note` with one fixed meaning |
| a small language inside | DSL macros | a vocabulary in the one declaration shape |
| compile-time values | `constexpr`, build scripts | pure with constant inputs is computed in the build |
| tables from data | code generators | a module of `const`, checked like code |
| same code for many types | templates, generics | generics; operations passed as values ([33](33-data-structures-and-algorithms.md)) |
| code from an outside schema | generators (OpenAPI, Protobuf) | `edit --import` writes a contract once, as source you own |
| code per platform | `#ifdef` | `app`, handlers, `@fast` pairs ([31](31-dropping-down.md)) |
| look at a type at run time | reflection | `view --json` while developing; derived serialisers when running |
| change code while it runs | monkey patching, hot swap | not possible |
| new operators and syntax | reader macros | not possible |

## How an expansion works

```slg
use vocab;

@allows(module)
kind archived(fields: List[Field]) -> List[Decl] = expand_archived;

@allows(entity)
note retain(days: 1..=3650) = retain_policy;
```

The function behind a kind receives the declaration as data and returns declarations as data. It is total, sees no names but its own, opens nothing, and cannot add a second writer to anyone's state. Then:

```text
$ spec-lang view orders.Cart --as expanded
```

prints the result as plain source, which can be read, tested, or pasted over the shorthand to change one thing. **A shorthand you can open is the difference between an abstraction and magic.**

| Guarantee | Why it matters to a model |
|---|---|
| same input, same expansion | one lookup is enough |
| errors point at what you wrote | not at generated code you have never seen |
| an upgrade that changes an expansion is a guarantee diff | no silent change of meaning |
| two expansions commute or the build stops | order of lines never matters |
| size is shown | a word that expands to 400 declarations says so |

## Small languages

A domain-specific language is a good idea with a bad habit: each one brings its own grammar, errors, editor and manual, and a model has seen little of it. A vocabulary keeps the good part. It is a small language **of meanings** in a grammar that never changes.

| Wish | Form |
|---|---|
| say it in the words of the domain | kinds and notes |
| a rule of the domain that must hold | laws in the vocabulary |
| nicer layout for tables, machines, schedules | a view: `view --as table`, `--as diagram`; never the source |
| a fluent chain | ordinary calls: `orders.filter(...).sort(...)` |
| a notation that already exists and models know well | tagged text, with a checker |

Tagged text is for languages that are older than this one and will outlive it: `re"..."`, `date"..."`, `url"..."`, and in vocabularies `sql"..."` or `wgsl"..."`. Three conditions: the tag has a checker that runs with the types, values go in as typed holes and never by joining strings, and the text has one known result type. Without a checker it is `foreign`, in its own file.

Opinion of the author of these docs, a model: I would rather learn thirty new words in a grammar I know than five words in a grammar I do not. New words I can look up; a new grammar I get subtly wrong.

## Generating code is the author's job now

Much metaprogramming existed because people disliked typing the same thing. A model does not mind. So the order is:

1. Write it out, if it is short and appears once or twice.
2. A `fn` or a generic, if the repetition is in the logic.
3. A `note`, if it is a policy on many declarations.
4. A `kind`, if it is a whole shape that keeps coming back, has laws of its own, and the third use is real.

A model proposes a kind by showing three existing places, the declaration that replaces them, and the tokens saved, its card included. It is accepted when the three places pass their old tests unchanged.

## Outside generators

Scripts may write `.slg` files, for example from a spreadsheet or another system's schema. The result is source: formatted, checked, reviewed, committed. The script is not part of the build, and the build never depends on it being run.

## Left out on purpose

| Not here | Because |
|---|---|
| user syntax, operators, precedence | every project becomes a dialect |
| arbitrary compile-time code | a build that can do anything can fail in any way, and a dependency could attack it |
| run-time reflection and evaluation of text | the checker cannot see it, and injected text becomes code |
| changing somebody else's declaration | two writers, no owner |
| conditional compilation | a program that was never checked in that shape |

## Limits

- Total expansions cannot express everything a macro can. A few real needs will have no form and will be written out or go `foreign`.
- A bad kind is still bad: a word with too many options, or a meaning nobody can guess.
- Vocabularies cost card tokens each time they are loaded, so a kind must pay for itself.
- Debugging an expansion means reading declarations as data, which nobody has tried yet.
- `edit --import` is a proposal.
