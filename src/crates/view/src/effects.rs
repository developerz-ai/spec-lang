//! `diagram --kind effects`: what each rule can read, write and send, and
//! which rules therefore cannot be reordered freely.
//!
//! The sets are read off the body and the guards, through every `fn` and
//! `rule` called ([`crate::facts::Facts::touch`]), and compared with the
//! rule's `@uses`. Two rules **conflict** when one writes a state the other
//! reads or writes: each is atomic, so nothing is torn, but the outcome
//! depends on which commits first. A rule that writes also conflicts with a
//! second call of itself. `out` is not counted: lines are sent after commit.

use crate::facts::{Facts, Kind};
use crate::graph::Graph;
use crate::json::Json;
use spec_lang_ir::DeclRef;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectRow {
    /// Qualified rule name.
    pub rule: String,
    /// `@uses(...)` as written.
    pub declared: Vec<String>,
    pub reads: Vec<String>,
    pub writes: Vec<String>,
    /// Outside effects: `out.line`.
    pub outside: Vec<String>,
    /// Read off the body but not in `@uses`.
    pub undeclared: Vec<String>,
    /// In `@uses` but not read off the body.
    pub unused: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    pub a: String,
    pub b: String,
    /// The states one writes and the other reads or writes.
    pub states: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Effects {
    /// Rules, states and `out`; edges `reads`, `writes`, `emits`. Contended states are hot.
    pub graph: Graph,
    pub rows: Vec<EffectRow>,
    pub conflicts: Vec<Conflict>,
}

pub fn effects(f: &Facts<'_>) -> Effects {
    let mut g = Graph::new("effects", "read sets, write sets and outside effects of every rule");
    let rules: Vec<usize> = (0..f.decls.len()).filter(|&i| f.decls[i].kind == Kind::Rule).collect();
    let touches: Vec<_> = rules.iter().map(|&i| f.touch(i)).collect();
    let mut rows = Vec::new();

    for &i in &rules {
        let d = &f.decls[i];
        g.node(&d.id, &d.name, "rule", &d.module, &d.qname);
    }
    for (i, d) in f.decls.iter().enumerate() {
        if d.kind == Kind::State && touches.iter().any(|t| t.reads.contains(&i) || t.writes.contains(&i)) {
            g.node(&d.id, &d.name, "state", &d.module, &d.qname);
        }
    }
    if touches.iter().any(|t| t.out) {
        g.node("fx_out", "out", "effect", "", "");
    }

    for (&i, t) in rules.iter().zip(&touches) {
        let d = &f.decls[i];
        for &s in &t.reads {
            g.edge(&d.id, &f.decls[s].id, "reads", "");
        }
        for &s in &t.writes {
            g.edge(&d.id, &f.decls[s].id, "writes", "");
        }
        if t.out {
            g.edge(&d.id, "fx_out", "emits", "line");
        }

        let declared = match d.r {
            DeclRef::Rule(id) => f.p.rule(id).uses.clone(),
            _ => Vec::new(),
        };
        // `@uses` names a state of the rule's own module by its bare name.
        let name = |s: usize| {
            if f.decls[s].module == d.module { f.decls[s].name.clone() } else { f.decls[s].qname.clone() }
        };
        let mut found: Vec<String> = Vec::new();
        for &s in t.reads.union(&t.writes) {
            found.push(format!("{}.{}", name(s), if t.writes.contains(&s) { "write" } else { "read" }));
        }
        if t.out {
            found.push("out.write".to_string());
        }
        let covers = |have: &str, need: &str| {
            have == need || need.strip_suffix(".read").is_some_and(|n| have.strip_suffix(".write") == Some(n))
        };
        rows.push(EffectRow {
            rule: d.qname.clone(),
            reads: t.reads.iter().map(|&s| f.decls[s].qname.clone()).collect(),
            writes: t.writes.iter().map(|&s| f.decls[s].qname.clone()).collect(),
            outside: if t.out { vec!["out.line".to_string()] } else { Vec::new() },
            undeclared: found.iter().filter(|x| !declared.iter().any(|d| covers(d, x))).cloned().collect(),
            unused: declared.iter().filter(|d| !found.iter().any(|x| covers(x, d))).cloned().collect(),
            declared,
        });
    }

    let mut conflicts = Vec::new();
    for a in 0..rules.len() {
        for b in a..rules.len() {
            let (ta, tb) = (&touches[a], &touches[b]);
            let mut states: Vec<usize> = Vec::new();
            for &s in &ta.writes {
                if tb.reads.contains(&s) || tb.writes.contains(&s) {
                    states.push(s);
                }
            }
            for &s in &tb.writes {
                if (ta.reads.contains(&s) || ta.writes.contains(&s)) && !states.contains(&s) {
                    states.push(s);
                }
            }
            states.sort_unstable();
            if !states.is_empty() {
                for &s in &states {
                    g.mark_hot(&f.decls[s].id);
                }
                conflicts.push(Conflict {
                    a: f.decls[rules[a]].qname.clone(),
                    b: f.decls[rules[b]].qname.clone(),
                    states: states.iter().map(|&s| f.decls[s].qname.clone()).collect(),
                });
            }
        }
    }
    Effects { graph: g, rows, conflicts }
}

impl Effects {
    /// Rules as nodes, one edge per conflicting pair, labelled with the states they share.
    pub fn conflict_graph(&self, f: &Facts<'_>) -> Graph {
        let mut g = Graph::new("effects.conflicts", "rules whose order matters");
        let short = |q: &str| q.rsplit('.').next().unwrap_or(q).to_string();
        for d in f.decls.iter().filter(|d| d.kind == Kind::Rule) {
            g.node(&d.id, &d.name, "rule", &d.module, &d.qname);
        }
        let id = |q: &str| f.decls.iter().find(|d| d.qname == q).map(|d| d.id.clone()).unwrap_or_default();
        for c in &self.conflicts {
            let states: Vec<String> = c.states.iter().map(|s| short(s)).collect();
            g.edge(&id(&c.a), &id(&c.b), "can race", &format!("on {}", states.join(", ")));
        }
        g
    }

    pub fn to_json(&self) -> Json {
        let rows = self.rows.iter().map(|r| {
            Json::obj([
                ("rule", Json::str(&r.rule)),
                ("uses", Json::strs(&r.declared)),
                ("reads", Json::strs(&r.reads)),
                ("writes", Json::strs(&r.writes)),
                ("outside", Json::strs(&r.outside)),
                ("undeclared", Json::strs(&r.undeclared)),
                ("unused", Json::strs(&r.unused)),
            ])
        });
        let conflicts = self
            .conflicts
            .iter()
            .map(|c| Json::obj([("a", Json::str(&c.a)), ("b", Json::str(&c.b)), ("states", Json::strs(&c.states))]));
        let Json::Obj(mut fields) = self.graph.to_json() else { return Json::Null };
        fields.push(("rules".to_string(), Json::Arr(rows.collect())));
        fields.push(("conflict_count".to_string(), Json::count(self.conflicts.len())));
        fields.push(("conflicts".to_string(), Json::Arr(conflicts.collect())));
        Json::Obj(fields)
    }
}
