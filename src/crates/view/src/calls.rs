//! `diagram --kind calls`: the declaration graph across modules.
//!
//! Edges: `calls` (a `fn` or `rule`), `reads` and `writes` (a `state` or
//! `const`), `constructs` (a record or a variant of a type), `emits` (an
//! outside effect). With tests and laws in scope also `given`, `sets`,
//! `traces` and `matches`. A `@cases` row is a test of its declaration and is
//! not an edge of it.

use crate::facts::{Facts, Kind, Phase, RefKind, Target};
use crate::graph::Graph;

/// Which declarations are sources of edges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// Types, constants, states, functions and rules: what the program is.
    Product,
    /// The product plus tests and laws: what checks it.
    WithChecks,
}

pub fn call_graph(f: &Facts<'_>, scope: Scope) -> Graph {
    let (view, title) = match scope {
        Scope::Product => ("calls", "declaration graph"),
        Scope::WithChecks => ("calls.checks", "declaration graph with tests and laws"),
    };
    let checks = scope == Scope::WithChecks;
    let source = |k: Kind| match k {
        Kind::Type | Kind::Const | Kind::State | Kind::Fn | Kind::Rule => true,
        Kind::Test | Kind::Law => checks,
        Kind::App => false,
    };
    let kept = |k: RefKind| match k {
        RefKind::Calls | RefKind::Reads | RefKind::Writes | RefKind::Constructs | RefKind::Emits => true,
        RefKind::Given | RefKind::Sets | RefKind::Traces | RefKind::Matches => checks,
        RefKind::Uses | RefKind::Exposes => false,
    };
    let mut edges: Vec<(usize, Target, RefKind)> = Vec::new();
    for (i, d) in f.decls.iter().enumerate().filter(|(_, d)| source(d.kind)) {
        for r in d.refs.iter().filter(|r| r.phase != Phase::Cases && kept(r.kind)) {
            let e = (i, r.to, r.kind);
            if r.to != Target::Decl(i) && !edges.contains(&e) {
                edges.push(e);
            }
        }
    }
    let mut g = Graph::new(view, title);
    for (i, d) in f.decls.iter().enumerate() {
        let always = matches!(d.kind, Kind::Const | Kind::State | Kind::Fn | Kind::Rule)
            || (checks && matches!(d.kind, Kind::Test | Kind::Law));
        let touched = edges.iter().any(|(from, to, _)| *from == i || *to == Target::Decl(i));
        if always || touched {
            g.node(&d.id, &d.name, d.kind.as_str(), &d.module, &d.qname);
        }
    }
    if edges.iter().any(|(_, to, _)| *to == Target::Out) {
        g.node("fx_out", "out", "effect", "", "");
    }
    for (from, to, kind) in edges {
        let to = match to {
            Target::Decl(i) => f.decls[i].id.as_str(),
            Target::Out => "fx_out",
        };
        g.edge(&f.decls[from].id, to, kind.as_str(), "");
    }
    g
}

/// One declaration with every edge into and out of it.
pub fn neighbourhood(g: &Graph, id: &str) -> Graph {
    let mut out = Graph::new(&g.view, format!("{} around {id}", g.title));
    let edges: Vec<_> = g.edges.iter().filter(|e| e.from == id || e.to == id).cloned().collect();
    for n in &g.nodes {
        if n.id == id || edges.iter().any(|e| e.from == n.id || e.to == n.id) {
            out.nodes.push(n.clone());
        }
    }
    out.edges = edges;
    out.mark_hot(id);
    out
}
