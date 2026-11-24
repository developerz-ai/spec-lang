//! `diagram --kind modules`: the `use` graph, who owns which state, and which
//! `app` puts which module's rules on which target.

use crate::facts::{Facts, Kind, info};
use crate::graph::Graph;
use crate::text::Ids;
use spec_lang_ir::{DeclRef, Store, UseItem};
use std::collections::BTreeMap;

pub fn module_graph(f: &Facts<'_>) -> Graph {
    let p = f.p;
    let mut g = Graph::new("modules", "modules, state ownership and targets");
    let mut ids = Ids::new();
    let mut module_id: BTreeMap<&str, String> = BTreeMap::new();
    for m in f.modules() {
        let id = ids.fresh(&format!("mod_{m}"));
        g.node(&id, m, "module", "", "");
        module_id.insert(m, id);
    }
    let mut vocab_id: BTreeMap<String, String> = BTreeMap::new();
    let mut target_id: BTreeMap<String, String> = BTreeMap::new();

    for m in p.modules.iter().filter(|m| !m.path.is_empty()) {
        let from = module_id[m.name.as_str()].clone();
        // One edge per used module, labelled with the names taken from it.
        let mut used: Vec<(String, Vec<String>)> = Vec::new();
        for u in &m.uses {
            match u {
                UseItem::Module { module, .. } => {
                    let name = p.module(*module).name.clone();
                    if !used.iter().any(|(n, _)| *n == name) {
                        used.push((name, Vec::new()));
                    }
                }
                UseItem::Decl { name, decl, .. } => {
                    let module = p.module(info(p, *decl).module).name.clone();
                    match used.iter_mut().find(|(n, _)| *n == module) {
                        Some((_, names)) => names.push(name.clone()),
                        None => used.push((module, vec![name.clone()])),
                    }
                }
                UseItem::Vocabulary { name, .. } => {
                    let id =
                        vocab_id.entry(name.clone()).or_insert_with(|| ids.fresh(&format!("vocab_{name}"))).clone();
                    g.node(&id, name, "vocabulary", "", "");
                    g.edge(&from, &id, "uses", "");
                }
            }
        }
        for (module, names) in used {
            if let Some(to) = module_id.get(module.as_str()) {
                g.edge(&from, to, "uses", &names.join(", "));
            }
        }
    }

    for d in &f.decls {
        let from = module_id[d.module.as_str()].clone();
        match d.r {
            DeclRef::State(_) => {
                g.node(&d.id, &d.name, "state", "", &d.qname);
                g.edge(&from, &d.id, "owns", "");
            }
            DeclRef::App(id) => {
                let app = p.app(id);
                g.node(&d.id, &d.name, "app", "", &d.qname);
                g.edge(&from, &d.id, "declares", "");
                // Rules exposed, counted per module that holds them.
                let mut per_module: Vec<(String, usize)> = Vec::new();
                for r in &app.rules {
                    let module = p.module(p.rule(*r).info.module).name.clone();
                    match per_module.iter_mut().find(|(m, _)| *m == module) {
                        Some((_, n)) => *n += 1,
                        None => per_module.push((module, 1)),
                    }
                }
                for (module, n) in per_module {
                    if let Some(to) = module_id.get(module.as_str()) {
                        g.edge(&d.id, to, "exposes", &format!("{n} rules"));
                    }
                }
                if let Some(target) = &app.target {
                    let tid = target_id
                        .entry(target.clone())
                        .or_insert_with(|| ids.fresh(&format!("target_{target}")))
                        .clone();
                    g.node(&tid, target, "target", "", "");
                    let mut how = Vec::new();
                    if let Some(bin) = &app.bin {
                        how.push(format!("bin {bin}"));
                    }
                    match &app.store {
                        Some(Store::File(path)) => how.push(format!("store file {path}")),
                        None => {}
                    }
                    g.edge(&d.id, &tid, "targets", &how.join(", "));
                }
            }
            _ => {}
        }
    }
    debug_assert!(f.decls.iter().all(|d| d.kind != Kind::State || g.has_node(&d.id)));
    g
}
