//! One Markdown page per view: counts, Mermaid blocks, the tables a reviewer
//! reads beside the picture, and what the view does not show.

use crate::calls::{Scope, call_graph};
use crate::class::class_diagram;
use crate::effects::effects;
use crate::facts::{Facts, Kind};
use crate::impact::impact;
use crate::modules::module_graph;
use crate::provenance::Provenance;
use crate::pseudo::pseudo_module;
use crate::sequence::test_sequences;
use crate::state::{FlowEdge, silent_writers, state_machines, unmoved_places};
use crate::text::{markdown as md, mermaid_block};
use std::fmt::Write as _;

fn head(title: &str, projection: &str) -> String {
    format!(
        "# {title}\n\n> **Generated.** Projection `{projection}` of the program's IR, printed by the `view` crate. \
         Do not edit: change the source and regenerate. Same source, same bytes.\n\n"
    )
}

fn limits(out: &mut String, items: &[&str]) {
    out.push_str("## Limits\n\n");
    for i in items {
        let _ = writeln!(out, "- {i}");
    }
}

fn code(names: &[String]) -> String {
    if names.is_empty() {
        "-".to_string()
    } else {
        names.iter().map(|n| format!("`{}`", md(n))).collect::<Vec<_>>().join(", ")
    }
}

pub fn class_page(f: &Facts<'_>) -> String {
    let d = class_diagram(f);
    let mut out = head("Class diagram", "diagram --kind class");
    let _ = writeln!(out, "**{} classes, {} relations.**\n", d.classes.len(), d.relations.len());
    out.push_str(&mermaid_block(&d.to_mermaid()));
    out.push_str("\n| Class | Kind | Declaration | Constraints |\n|---|---|---|---|\n");
    for c in &d.classes {
        let _ =
            writeln!(out, "| `{}` | {} | `{}` | {} |", md(&c.name), c.stereotype, md(&c.decl), code(&c.constraints));
    }
    out.push_str(
        "\nA note `{ claim }` is a constraint every value of the type satisfies. A filled diamond is a field \
         (the record holds the value); a hollow diamond is a module's `state`, labelled with the map key in brackets.\n\n",
    );
    limits(
        &mut out,
        &[
            "Functions are not drawn; a claim that calls one shows the call as text only.",
            "A type named only through `Trace[...]` or a rule set has no relation line.",
            "Multiplicity is read off the type (`Option`, `List`, `Set`, `Map`), not off claims that bound a size.",
        ],
    );
    out
}

pub fn state_page(f: &Facts<'_>, flows: &[FlowEdge]) -> String {
    let machines = state_machines(f, flows);
    let mut out = head("State diagrams", "diagram --kind state");
    let _ = writeln!(out, "**{} machines.** An edge reads `rule [guards] / effects`.\n", machines.len());
    for m in &machines {
        let _ = writeln!(out, "## `{}`\n", md(&m.name));
        let _ = writeln!(out, "**{} nodes, {} edges.**\n", m.node_count(), m.transitions.len());
        out.push_str(&mermaid_block(&m.to_mermaid()));
        out.push_str("\n| From | To | Rule | Other guards | Effects | Read from |\n|---|---|---|---|---|---|\n");
        for t in &m.transitions {
            let end = |e: &crate::state::End| match e {
                crate::state::End::Outside => "outside".to_string(),
                crate::state::End::Any => "any state".to_string(),
                crate::state::End::State(s) => format!("`{}`", md(s)),
                crate::state::End::Computed => "computed".to_string(),
            };
            let _ = writeln!(
                out,
                "| {} | {} | `{}` | {} | {} | {} |",
                end(&t.from),
                end(&t.to),
                md(&t.rule),
                code(&t.guards),
                code(&t.effects),
                t.source
            );
        }
        out.push('\n');
    }
    let silent = silent_writers(f, &machines);
    if !silent.is_empty() {
        let _ = writeln!(out, "Rules that write state and move no machine: {}.\n", code(&silent));
    }
    let unmoved = unmoved_places(f);
    if !unmoved.is_empty() {
        let _ = writeln!(
            out,
            "Sum-typed places that are set when created and never moved, so not drawn: {}.\n",
            code(&unmoved)
        );
    }
    limits(
        &mut out,
        &[
            "A move is found only where a rule assigns a variant by name; a value passed in or computed is drawn as `(computed)`.",
            "Where a move starts is read from a guard `place == Variant` or `place.is(Variant)` on the same place text; with no such guard the move is drawn from every state.",
            "An exit is drawn from any state: which entries a `remove` in a loop takes is not decided here.",
            "`flow` declarations are drawn only when passed in as data; the IR does not carry them yet.",
        ],
    );
    out
}

pub fn sequence_page(f: &Facts<'_>) -> String {
    let seqs = test_sequences(f);
    let mut out = head("Sequence diagrams", "diagram --kind sequence");
    let _ = writeln!(out, "**{} tests.** One diagram per `test`, read off its body; nothing was run.\n", seqs.len());
    out.push_str("| Test | Participants | Messages |\n|---|---|---|\n");
    for s in &seqs {
        let _ = writeln!(out, "| `{}` | {} | {} |", md(&s.title), s.participants.len(), s.message_count());
    }
    out.push('\n');
    for s in &seqs {
        let _ = writeln!(out, "## `{}`\n", md(&s.title));
        out.push_str(&mermaid_block(&s.to_mermaid()));
        out.push('\n');
    }
    limits(
        &mut out,
        &[
            "The arrows to state and `out` are everything the called rule can touch, not what this call did touch: no rule is executed.",
            "A refused call shows the reads of the rule's guards only.",
            "Calls a rule makes to functions are not drawn; see the call graph.",
            "A note longer than 96 characters is cut and marked with `…`.",
        ],
    );
    out
}

pub fn modules_page(f: &Facts<'_>) -> String {
    let g = module_graph(f);
    let mut out = head("Modules", "diagram --kind modules");
    let _ = writeln!(out, "**{}.**\n", g.counts().sentence());
    out.push_str(&mermaid_block(&g.to_mermaid("LR")));
    out.push('\n');
    limits(
        &mut out,
        &[
            "Events are not drawn: the IR has no `event` yet.",
            "A `uses` edge lists the names imported one by one; an edge with no names is a whole-module `use`.",
            "Places and handlers of an `app` are not drawn; only `@target`, `@bin` and `@store` are read.",
        ],
    );
    out
}

pub fn effects_page(f: &Facts<'_>) -> String {
    let e = effects(f);
    let mut out = head("Effects", "diagram --kind effects");
    let _ = writeln!(
        out,
        "**{}.** A state with a red border is written by one rule and read or written by another.\n",
        e.graph.counts().sentence()
    );
    out.push_str(&mermaid_block(&e.graph.to_mermaid("LR")));
    out.push_str("\n| Rule | `@uses` | Reads | Writes | Outside | Not in `@uses` | In `@uses`, not found |\n|---|---|---|---|---|---|---|\n");
    for r in &e.rows {
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {} | {} | {} | {} |",
            md(&r.rule),
            code(&r.declared),
            code(&r.reads),
            code(&r.writes),
            code(&r.outside),
            code(&r.undeclared),
            code(&r.unused)
        );
    }
    let cg = e.conflict_graph(f);
    let _ = writeln!(
        out,
        "\n## Rules whose order matters\n\n**{} pairs** ({} nodes, {} edges). One rule writes a state the other reads or writes. \
         Each rule is atomic, so nothing is torn; the result depends on which commits first. A loop on one rule is two calls of it.\n",
        e.conflicts.len(),
        cg.nodes.len(),
        cg.edges.len()
    );
    out.push_str(&mermaid_block(&cg.to_mermaid("LR")));
    out.push_str("\n| Rule | Rule | Shared state |\n|---|---|---|\n");
    for c in &e.conflicts {
        let _ = writeln!(out, "| `{}` | `{}` | {} |", md(&c.a), md(&c.b), code(&c.states));
    }
    out.push('\n');
    limits(
        &mut out,
        &[
            "Sets are per state, not per key: two rules that touch different entries of one map are still listed as a pair.",
            "Sets are what a rule can touch on some path, not what every call touches.",
            "`out` is left out of the pairs: lines are sent after commit.",
            "`@uses` is compared by name; the IR records it and does not check it.",
        ],
    );
    out
}

pub fn calls_page(f: &Facts<'_>) -> String {
    let product = call_graph(f, Scope::Product);
    let checks = call_graph(f, Scope::WithChecks);
    let mut out = head("Call graph", "diagram --kind calls");
    let _ = writeln!(out, "## The product\n\n**{}.**\n", product.counts().sentence());
    out.push_str(&mermaid_block(&product.to_mermaid("LR")));
    let _ = writeln!(out, "\n## With tests and laws\n\n**{}.**\n", checks.counts().sentence());
    out.push_str(&mermaid_block(&checks.to_mermaid("LR")));
    out.push('\n');
    limits(
        &mut out,
        &[
            "Core library methods (`filter`, `sort`, `size`) are not declarations and are not drawn.",
            "A type is drawn only where it is constructed or matched; a type in a signature is an `impact` edge, not a call edge.",
            "A law's reads of `before` and `after` worlds are not edges: the IR resolves them when the law runs.",
            "`@cases` rows are not edges.",
        ],
    );
    out
}

/// Every declaration something depends on, each with its own graph.
pub fn impact_page(f: &Facts<'_>, provenance: &[Provenance]) -> String {
    let mut out = head("Impact", "diagram --kind impact <decl>");
    out.push_str("What may behave differently when a declaration changes: everything that names it, directly or through something that does. In a graph an arrow points at what is depended on, and is drawn where it leads one step closer to the declaration asked about: every shortest reason, not every reference.\n\n");
    out.push_str("| Declaration | Kind | Direct | Functions | Rules | Tests | Laws | `@cases` | Apps | All | Generated files |\n|---|---|---|---|---|---|---|---|---|---|---|\n");
    let all: Vec<_> = (0..f.decls.len()).map(|i| impact(f, i, provenance)).collect();
    for (i, x) in all.iter().enumerate() {
        let _ = writeln!(
            out,
            "| `{}` | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            md(&x.target),
            f.decls[i].kind.as_str(),
            x.direct.len(),
            x.of(Kind::Fn).len(),
            x.of(Kind::Rule).len(),
            x.of(Kind::Test).len(),
            x.of(Kind::Law).len(),
            x.cases.len(),
            x.of(Kind::App).len(),
            x.total(),
            x.files.len()
        );
    }
    out.push('\n');
    for x in all.iter().filter(|x| x.total() > 0) {
        let _ = writeln!(out, "## `{}`\n", md(&x.target));
        let _ = writeln!(out, "**{}.**\n", x.graph.counts().sentence());
        out.push_str(&mermaid_block(&x.graph.to_mermaid("RL")));
        let _ = writeln!(out, "\n- Tests: {}", code(x.of(Kind::Test)));
        let _ = writeln!(out, "- Laws: {}", code(x.of(Kind::Law)));
        let _ = writeln!(out, "- `@cases` of: {}", code(&x.cases));
        if !x.files.is_empty() {
            let files: Vec<String> = x.files.iter().map(|g| format!("{} {}", g.backend, g.at)).collect();
            let _ = writeln!(out, "- Generated: {}", code(&files));
        }
        out.push('\n');
    }
    limits(
        &mut out,
        &[
            "Dependence is by name. A rule that reads a state another rule writes does not depend on that rule here; the effects view lists such pairs.",
            "A law reaches state through the rules it traces; a law over no trace of a writer is not listed for that state.",
            "Generated files are listed only when a `provenance.json` was passed in.",
            "This is the cone of what can change, not a claim that it does.",
        ],
    );
    out
}

pub fn pseudo_page(f: &Facts<'_>) -> String {
    let mut out = head("Pseudocode", "pseudo");
    out.push_str("Every function and rule as textbook pseudocode: `←` assigns, `=` compares, `require p else fail E` is a guard checked before the body, `emit` asks for an outside effect after commit.\n\n");
    for m in f.modules() {
        let text = pseudo_module(f, m);
        if text.is_empty() {
            continue;
        }
        let _ = writeln!(out, "## `{}`\n\n```text\n{}```\n", md(m), text);
    }
    limits(
        &mut out,
        &[
            "Types, tests, laws and apps are not printed here.",
            "Core library methods keep their spec-lang names (`filter`, `sort`); they are not expanded into loops.",
            "A reading aid, never an input: it is not parsed back.",
        ],
    );
    out
}
