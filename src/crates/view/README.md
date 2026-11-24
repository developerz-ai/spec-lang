# view

> **Runs as a library.** The projections of `spec-lang view` ([docs/idea/10](../../../docs/idea/10-compiler-and-ir.md), "Projections"): UML diagrams, codebase graphs, pseudocode and a static explorer, computed from the IR. `spec-lang view` is not wired to it yet; the functions below are what the command line calls. Its output for the todo program is committed in [`pocs/todo-app/docs/views/`](../../../pocs/todo-app/docs/views/README.md).

Humans read projections; only source is edited. Every function here is a pure query over a `spec_lang_ir::Program`: it runs no rule, reads no file, and gives the same bytes for the same program.

## API for the command line

```rust
use view::{DiagramKind, Format, Inputs};

// spec-lang view --as diagram --kind <kind> [<subject>] [--format mermaid|plantuml|json|markdown]
let kind = DiagramKind::parse("state")?;            // class state sequence modules effects calls impact
let format = Format::parse("mermaid")?;             // mermaid plantuml json markdown (md)
let text: String = view::diagram(&program, kind, subject, format, Inputs::default())?;

// spec-lang view --as pseudo [<module>]
let text: String = view::pseudo(&program, module)?;

// spec-lang view --as site --out <dir>, and every committed view of a project
let files: Vec<view::File> = view::bundle(&program, "todo-app", "<command that regenerates>", Inputs::default());
view::write_files(out_dir, &files)?;                // the only function that touches the disk

// a law's counterexample, from the engine: the steps are data, nothing is run
let seq = view::sequence::trace_sequence(&view::sequence::Trace { title, actor, given, steps, verdict });
let mermaid = seq.to_mermaid();                     // also to_plantuml(), to_json()
```

| Call | Returns | Errors (`ViewError`, all `Display`) |
|---|---|---|
| `diagram(program, kind, subject, format, inputs)` | one text | `NoPrinter` (PlantUML of a graph view), `UnknownSubject` (with candidates), `NeedsSubject` (Mermaid holds one diagram; the view has several) |
| `pseudo(program, module)` | pseudocode of one module, or of all | `UnknownSubject` |
| `bundle(program, title, regenerate, inputs)` | every file of a views folder, `README.md` first | none |
| `site::site(&Facts::of(program), &SiteOptions)` | the explorer's pages only | none |
| `sequence::trace_sequence(&Trace)` | a `Sequence` | none |
| `provenance::Provenance::parse(text, backend, href)` | a generated project's `provenance.json` | a `String` naming the span |

`subject` picks one diagram where a kind has several: a machine (`todo.model.Task.status`) for `state`, a test for `sequence`, a declaration for `impact`; for `class` and `calls` it narrows the diagram to one declaration and its neighbours. A subject is a qualified name, a dotted suffix of it (`board.done`), or a bare name that is unique.

`Inputs { provenance, flows }` is what a view reads besides the program: parsed `provenance.json` files (for `impact` and the site's "Emitted code"), and `flow` edges (see Limits). Both default to empty.

Suggested exit codes: a `ViewError` is a usage error (2); `LoadFailure` from the IR stays what `test` makes it.

## Kinds

| Kind | Draws | Formats |
|---|---|---|
| `class` | types as classes, fields with defaults, sums, claims as `{ constraint }` notes, a module's `state` as attributes and associations with multiplicity, its rules as operations | Mermaid, PlantUML, JSON, Markdown |
| `state` | one machine per sum-typed place: moves read off rules that assign a variant, sources read off guards, entry on construction, exit on removal; edges `rule [guards] / effects`. `flow` edges when passed in | Mermaid, PlantUML, JSON, Markdown |
| `sequence` | one diagram per `test`: test, rules, state, `out`, in the order written; `trace_sequence` for a recorded trace | Mermaid, PlantUML, JSON, Markdown |
| `modules` | `use` graph, state ownership, apps, targets | Mermaid, JSON, Markdown |
| `effects` | each rule's read set, write set and outside effects, compared with `@uses`; the pairs of rules whose order matters | Mermaid, JSON, Markdown |
| `calls` | the declaration graph: calls, reads, writes, constructs, emits; with tests and laws also given, sets, traces, matches | Mermaid, JSON, Markdown |
| `impact <decl>` | everything that names a declaration, directly or through something that does; generated files from `provenance.json` | Mermaid, JSON, Markdown |
| `pseudo` | functions and rules as textbook pseudocode; guards as `require p else fail E` | text |
| site | index, a page per diagram kind, per module and per declaration | HTML |

Every JSON graph has `counts` (nodes and edges, in total and by kind), `nodes` and `edges`. A Markdown page states the same counts above each diagram, so a diagram can be approved as an exact graph ([docs/idea/08](../../../docs/idea/08-approval-and-anti-cheating.md)).

## Layout

| File | Holds |
|---|---|
| `src/lib.rs` | the API above |
| `src/facts.rs` | what every view reads off the IR: declarations by qualified name, and for each what it calls, reads, writes, constructs and names, with the phase (signature, guard, body, cases) |
| `src/graph.rs` `src/json.rs` `src/text.rs` | the node-and-edge model, its Mermaid and JSON printers, escaping for each format |
| `src/class.rs` `src/state.rs` `src/sequence.rs` | the three UML models, each with Mermaid, PlantUML and JSON printers |
| `src/modules.rs` `src/effects.rs` `src/calls.rs` `src/impact.rs` | the graph views |
| `src/pseudo.rs` `src/pages.rs` `src/site.rs` `src/provenance.rs` | pseudocode, the Markdown pages, the explorer, the reader of `provenance.json` |
| `examples/generate.rs` | writes a views folder; stands in for the command line |
| `tests/golden.rs` | the committed views of the todo program are byte for byte what the crate prints |
| `tests/lamps.rs` | every view on a 40-line program, checked by eye |

## Run it

```bash
cd src
VIEW_BLESS=1 cargo test -p view --test golden      # regenerate pocs/todo-app/docs/views, then compare
heavy cargo test -p view                           # fails when the committed views are stale
cargo fmt --check && cargo clippy -p view --all-targets -- -D warnings
cargo run -p view --example generate -- <src dir> <out dir> --title <name>   # any other project
```

To the last command add `--provenance rust=<project>/provenance.json=<href of the project from site/>` to list and link emitted files.

## What the IR provided

Everything is read from `spec-lang-ir`; this crate does no parsing and no name resolution of its own.

| Used | For |
|---|---|
| declaration tables, `DeclInfo` (module, name, span, `pub`, `@why`) | every view |
| resolved `ExprKind` (`State`, `Call { callee }`, `Variant`, `Lambda`), `Place`, `Pattern` | reads, writes, calls, constructs; pseudocode |
| `RuleDecl.guards`, `uses`, `triggers` | state sources, edge labels, the `@uses` comparison, sequences of refusals |
| `Ty` with `Where` claims, `TypeDef` | class diagram, the type of a place |
| `TestDecl.given`, `body`; `LawDecl` params (`Ty::Trace`) | sequences, impact |
| `AppDecl` (`rules`, `target`, `bin`, `store`), `Module.uses` | modules view |
| `Program::snippet` and `location` | guards, claims and defaults as written; `file:line` |

What it did not provide is in [IR-NEEDS.md](IR-NEEDS.md). Matches over IR enums are exhaustive on purpose: a new expression or statement kind is a compile error here, not a silently missing edge.

## Verified

- Mermaid: all 91 distinct diagrams of the todo views (Markdown pages and site) render with `@mermaid-js/mermaid-cli` 11.4.2, which bundles Mermaid 11.17.2, the version the site pins. Checked by hand on 2026-11-24, not in the gate.
- PlantUML: `class.puml`, `state.puml` and `sequence.puml` of the todo views render with PlantUML 1.2024.8 (12 diagrams, no syntax error). Checked by hand, not in the gate.
- The explorer: one declaration page opened in headless Chrome; the diagrams drew. No other browser was tried.

## Limits

- **Not wired to `spec-lang view`.** The command line does not call this crate yet; `examples/generate.rs` stands in.
- **Static sets, not runs.** Read and write sets are what a rule can touch through everything it calls, per state and not per key. A sequence diagram of a test is its written steps with those sets, not an execution.
- **State machines are read off a narrow shape**: `place = Variant` with a guard `place == Variant` or `place.is(Variant)` on the same place text. A move through a local, a `with`, or a helper `fn` is not found, or is drawn as `(computed)`.
- **No `flow`, `event`, `task` or `handler`**: the IR does not carry them. `flow` edges can be passed in as data and are drawn; nothing produces them yet.
- **Impact is by name.** A law reaches state through the rules it traces; `Trace[Change]` is expanded by the IR, so the alias `Change` shows no dependents.
- **Projections named in docs/idea/10 and not built here**: `expanded`, `guarantees`, `explain`, `policy`, `plan`, `ts`, `rust`. They need the ledger, the plan or a backend printer.
- **Mermaid and PlantUML validity is checked by hand**, not by a test: the gate has no browser and no Java.
- **The explorer loads Mermaid from a pinned CDN URL.** Without it the pages show the diagram text. `SiteOptions::mermaid_src` can point at a local copy; none is shipped.
- **Layout is the renderer's.** The call graph of a larger program will be hard to read as one picture; the per-declaration pages and the JSON are the usable form.
