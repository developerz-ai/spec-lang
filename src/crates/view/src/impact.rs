//! `diagram --kind impact <decl>`: what depends on a declaration.
//!
//! A declaration depends on another when it names it: calls, reads, writes,
//! constructs, matches, has it in a signature, traces it, starts from it,
//! exposes it. The impact of a declaration is everything that reaches it along
//! those edges: the rules, tests, laws and apps that may behave differently
//! when it changes, and the generated files made from any of them.

use crate::facts::{Facts, Kind, Phase, Target};
use crate::graph::Graph;
use crate::json::Json;
use crate::provenance::Provenance;
use crate::text::Ids;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Impact {
    /// Qualified name of the declaration asked about.
    pub target: String,
    /// Declarations that name the target themselves (qualified, source order).
    pub direct: Vec<String>,
    /// Every declaration that reaches the target, by kind (qualified, source order).
    pub by_kind: Vec<(Kind, Vec<String>)>,
    /// Functions whose `@cases` rows depend on the target.
    pub cases: Vec<String>,
    /// Generated files made from the target or from anything that depends on it.
    pub files: Vec<ImpactFile>,
    /// The target (hot), its dependents, the files. An edge points at what is
    /// depended on, and is drawn where it leads one step closer to the target:
    /// every shortest reason, not every reference among the dependents.
    pub graph: Graph,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImpactFile {
    pub backend: String,
    /// `src/board/done.rs:1-12`.
    pub at: String,
    /// What the span is, as the emitter named it; may be empty.
    pub role: String,
    /// The declaration it was generated from.
    pub decl: String,
}

/// Every declaration that reaches `target`, with how many steps away it is; the target itself is at 0.
fn distances(f: &Facts<'_>, target: usize) -> BTreeMap<usize, usize> {
    let mut dist = BTreeMap::from([(target, 0)]);
    let mut level = vec![target];
    let mut steps = 0;
    while !level.is_empty() {
        steps += 1;
        let mut next = Vec::new();
        for (i, d) in f.decls.iter().enumerate() {
            let names =
                d.refs.iter().any(|r| r.phase != Phase::Cases && matches!(r.to, Target::Decl(t) if level.contains(&t)));
            if names && !dist.contains_key(&i) {
                dist.insert(i, steps);
                next.push(i);
            }
        }
        level = next;
    }
    dist
}

/// The indexes of every declaration that reaches `target`, itself excluded.
pub fn dependents(f: &Facts<'_>, target: usize) -> BTreeSet<usize> {
    distances(f, target).into_keys().filter(|&i| i != target).collect()
}

pub fn impact(f: &Facts<'_>, target: usize, provenance: &[Provenance]) -> Impact {
    let t = &f.decls[target];
    let dist = distances(f, target);
    let all: BTreeSet<usize> = dist.keys().copied().filter(|&i| i != target).collect();
    let names = |i: usize| f.decls[i].refs.iter().any(|r| r.phase != Phase::Cases && r.to == Target::Decl(target));
    let in_scope = |i: usize| i == target || all.contains(&i);

    let mut by_kind = Vec::new();
    for kind in [Kind::Type, Kind::Const, Kind::State, Kind::Fn, Kind::Rule, Kind::Test, Kind::Law, Kind::App] {
        let of: Vec<String> =
            all.iter().filter(|&&i| f.decls[i].kind == kind).map(|&i| f.decls[i].qname.clone()).collect();
        if !of.is_empty() {
            by_kind.push((kind, of));
        }
    }
    let has_cases = |i: usize| f.decls[i].refs.iter().any(|r| r.phase == Phase::Cases) || is_fn_with_cases(f, i);
    let cases: Vec<String> = (0..f.decls.len())
        .filter(|&i| {
            let through_rows = f.decls[i]
                .refs
                .iter()
                .any(|r| r.phase == Phase::Cases && matches!(r.to, Target::Decl(x) if in_scope(x)));
            has_cases(i) && (in_scope(i) || through_rows)
        })
        .map(|i| f.decls[i].qname.clone())
        .collect();

    let mut g = Graph::new("impact", format!("what depends on {}", t.qname));
    for (_, d) in f.decls.iter().enumerate().filter(|(i, _)| in_scope(*i)) {
        g.node(&d.id, &d.name, d.kind.as_str(), &d.module, &d.qname);
    }
    g.mark_hot(&t.id);
    for &i in &all {
        for r in f.decls[i].refs.iter().filter(|r| r.phase != Phase::Cases) {
            // Drawn where it is a shortest reason: the edge leads one step closer to the target.
            if let Target::Decl(x) = r.to
                && dist.get(&x).is_some_and(|dx| dx + 1 == dist[&i])
            {
                g.edge(&f.decls[i].id, &f.decls[x].id, r.kind.as_str(), "");
            }
        }
    }

    let mut files = Vec::new();
    let mut ids = Ids::new();
    for n in &g.nodes {
        ids.fresh(&n.id);
    }
    for prov in provenance {
        for (_, d) in f.decls.iter().enumerate().filter(|(i, _)| in_scope(*i)) {
            for span in prov.of_decl(&d.qname) {
                let at = span.text();
                let id = ids.fresh(&format!("file_{}_{at}", prov.backend));
                g.node(&id, &at, "file", &format!("generated: {}", prov.backend), "");
                g.edge(&id, &d.id, "generated from", "");
                files.push(ImpactFile {
                    backend: prov.backend.clone(),
                    at,
                    role: span.role.clone(),
                    decl: d.qname.clone(),
                });
            }
        }
    }

    Impact {
        target: t.qname.clone(),
        direct: (0..f.decls.len()).filter(|&i| i != target && names(i)).map(|i| f.decls[i].qname.clone()).collect(),
        by_kind,
        cases,
        files,
        graph: g,
    }
}

fn is_fn_with_cases(f: &Facts<'_>, i: usize) -> bool {
    match f.decls[i].r {
        spec_lang_ir::DeclRef::Fn(id) => !f.p.func(id).cases.is_empty(),
        spec_lang_ir::DeclRef::Test(id) => !f.p.test(id).cases.is_empty(),
        _ => false,
    }
}

impl Impact {
    pub fn of(&self, kind: Kind) -> &[String] {
        self.by_kind.iter().find(|(k, _)| *k == kind).map_or(&[], |(_, v)| v.as_slice())
    }

    /// How many declarations depend on the target.
    pub fn total(&self) -> usize {
        self.by_kind.iter().map(|(_, v)| v.len()).sum()
    }

    /// With `graph: false`, the lists and the graph's counts only.
    pub fn to_json(&self, graph: bool) -> Json {
        let files = self.files.iter().map(|x| {
            Json::obj([
                ("backend", Json::str(&x.backend)),
                ("at", Json::str(&x.at)),
                ("role", Json::str(&x.role)),
                ("decl", Json::str(&x.decl)),
            ])
        });
        let mut fields =
            vec![("target".to_string(), Json::str(&self.target)), ("direct".to_string(), Json::strs(&self.direct))];
        let mut dependents = Vec::new();
        for (kind, names) in &self.by_kind {
            dependents.push((format!("{}s", kind.as_str()), Json::strs(names)));
        }
        fields.push(("dependents".to_string(), Json::Obj(dependents)));
        fields.push(("cases".to_string(), Json::strs(&self.cases)));
        fields.push(("generated".to_string(), Json::Arr(files.collect())));
        if graph {
            fields.push(("graph".to_string(), self.graph.to_json()));
        } else {
            fields.push(("counts".to_string(), self.graph.counts().to_json()));
        }
        Json::Obj(fields)
    }
}
