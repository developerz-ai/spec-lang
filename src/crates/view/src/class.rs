//! `diagram --kind class`: types as classes, claims as constraints, state as associations.
//!
//! | Source | Drawn as |
//! |---|---|
//! | `type T(...)` | a `record` class, one attribute per field with its default |
//! | `type T = A \| B(x: X)` | an `enumeration` (no fields) or `sum` class, one line per variant |
//! | `type T = Base where claim` | a `type` class holding `Base`, each claim a UML constraint `{ claim }` |
//! | `type T = rule \| rule` | a `rule set` class |
//! | a field of a declared type | a composition, `0..1` under `Option`, `0..*` under `List`, `Set` or `Map` |
//! | a module with `state` | a `module` class: plain states are attributes, rules are operations |
//! | `state s: Map[K, V]`, `Set[V]`, `List[V]`, `Option[V]` | an aggregation from the module to `V`, labelled `s [K]` |

use crate::facts::{Facts, Kind};
use crate::json::Json;
use crate::text;
use spec_lang_ir::{DeclRef, Field, Param, Ty, TypeDef, TypeId};
use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassDiagram {
    pub classes: Vec<Class>,
    pub relations: Vec<Relation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Class {
    pub id: String,
    pub name: String,
    /// `record`, `enumeration`, `sum`, `type`, `rule set` or `module`.
    pub stereotype: String,
    /// The qualified name of the declaration, or the module name of a `module` class.
    pub decl: String,
    pub attributes: Vec<String>,
    pub operations: Vec<String>,
    /// Claims on the type, each as written.
    pub constraints: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelKind {
    /// A field holds the value.
    Composition,
    /// A module's state holds the values.
    Aggregation,
    /// An alias names the type.
    Alias,
}

impl RelKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RelKind::Composition => "composition",
            RelKind::Aggregation => "aggregation",
            RelKind::Alias => "alias",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Relation {
    pub from: String,
    pub to: String,
    pub kind: RelKind,
    pub label: String,
    /// Multiplicity at the `to` end: `1`, `0..1` or `0..*`.
    pub mult: String,
}

/// Every declared type a type mentions, with the multiplicity it is held in
/// and the map key it is found by.
fn targets(ty: &Ty, mult: &str, key: &str, f: &Facts<'_>, out: &mut Vec<(TypeId, String, String)>) {
    match ty {
        Ty::Unit | Ty::Bool | Ty::Int | Ty::Text | Ty::Range { .. } | Ty::Trace(_) => {}
        Ty::Named(id) => out.push((*id, mult.to_string(), key.to_string())),
        Ty::Option(t) => targets(t, if mult == "1" { "0..1" } else { mult }, key, f, out),
        Ty::List(t) | Ty::Set(t) => targets(t, "0..*", key, f, out),
        Ty::Map(k, v) => targets(v, "0..*", &format!(" [{}]", f.ty_text(k)), f, out),
        Ty::Tuple(ts) => ts.iter().for_each(|t| targets(t, mult, key, f, out)),
        Ty::Where(base, _) => targets(base, mult, key, f, out),
    }
}

/// The claims of `T where a where b`, outermost last, and the type under them.
fn claims<'a>(ty: &'a Ty, f: &Facts<'_>, out: &mut Vec<String>) -> &'a Ty {
    match ty {
        Ty::Where(base, claim) => {
            let inner = claims(base, f, out);
            out.push(f.src(claim.expr.span));
            inner
        }
        t => t,
    }
}

fn field_text(x: &Field, f: &Facts<'_>) -> String {
    match &x.default {
        Some(d) => format!("{}: {} = {}", x.name, f.ty_text(&x.ty), f.src(d.span)),
        None => format!("{}: {}", x.name, f.ty_text(&x.ty)),
    }
}

/// `name: T = default, ...` of a parameter list.
pub fn params_text(params: &[Param], f: &Facts<'_>) -> String {
    let one = |x: &Param| match &x.default {
        Some(d) => format!("{}: {} = {}", x.name, f.ty_text(&x.ty), f.src(d.span)),
        None => format!("{}: {}", x.name, f.ty_text(&x.ty)),
    };
    params.iter().map(one).collect::<Vec<_>>().join(", ")
}

/// The class diagram of a whole program.
pub fn class_diagram(f: &Facts<'_>) -> ClassDiagram {
    let p = f.p;
    let mut d = ClassDiagram { classes: Vec::new(), relations: Vec::new() };
    let class_id = |id: TypeId| f.index_of(DeclRef::Type(id)).map(|i| f.decls[i].id.clone());
    let mut ids = text::Ids::new();
    for decl in &f.decls {
        ids.fresh(&decl.id);
    }

    for (i, decl) in f.decls.iter().enumerate() {
        let DeclRef::Type(tid) = decl.r else { continue };
        let mut c = Class {
            id: decl.id.clone(),
            name: f.short(i).to_string(),
            stereotype: String::new(),
            decl: decl.qname.clone(),
            attributes: Vec::new(),
            operations: Vec::new(),
            constraints: Vec::new(),
        };
        let mut held = Vec::new();
        let mut relate = |label: &str, ty: &Ty, kind: RelKind, d: &mut ClassDiagram| {
            held.clear();
            targets(ty, "1", "", f, &mut held);
            for (to, mult, key) in &held {
                if let Some(to) = class_id(*to) {
                    let label = format!("{label}{key}");
                    d.relations.push(Relation { from: decl.id.clone(), to, kind, label, mult: mult.clone() });
                }
            }
        };
        match &p.ty(tid).def {
            TypeDef::Alias(ty) => {
                c.stereotype = "type".to_string();
                let base = claims(ty, f, &mut c.constraints);
                c.attributes.push(f.ty_text(base));
                relate("is", base, RelKind::Alias, &mut d);
            }
            TypeDef::Record(fields) => {
                c.stereotype = "record".to_string();
                for x in fields {
                    c.attributes.push(field_text(x, f));
                    relate(&x.name, &x.ty, RelKind::Composition, &mut d);
                }
            }
            TypeDef::Sum(variants) => {
                let plain = variants.iter().all(|v| v.fields.is_empty());
                c.stereotype = if plain { "enumeration" } else { "sum" }.to_string();
                for v in variants {
                    if v.fields.is_empty() {
                        c.attributes.push(v.name.clone());
                    } else {
                        let inner: Vec<String> = v.fields.iter().map(|x| field_text(x, f)).collect();
                        c.attributes.push(format!("{}({})", v.name, inner.join(", ")));
                    }
                    for x in &v.fields {
                        relate(&format!("{}.{}", v.name, x.name), &x.ty, RelKind::Composition, &mut d);
                    }
                }
            }
            TypeDef::Rules(rules) => {
                c.stereotype = "rule set".to_string();
                c.attributes = rules.iter().map(|r| f.short_of(DeclRef::Rule(*r))).collect();
            }
        }
        d.classes.push(c);
    }

    for module in f.modules() {
        let states: Vec<usize> =
            (0..f.decls.len()).filter(|&i| f.decls[i].module == module && f.decls[i].kind == Kind::State).collect();
        if states.is_empty() {
            continue;
        }
        let id = ids.fresh(module);
        let mut c = Class {
            id: id.clone(),
            name: module.to_string(),
            stereotype: "module".to_string(),
            decl: module.to_string(),
            attributes: Vec::new(),
            operations: Vec::new(),
            constraints: Vec::new(),
        };
        for i in states {
            let DeclRef::State(sid) = f.decls[i].r else { continue };
            let s = p.state(sid);
            let collection = matches!(f.underlying(&s.ty), Ty::Map(..) | Ty::Set(_) | Ty::List(_) | Ty::Option(_));
            let mut held = Vec::new();
            if collection {
                targets(f.underlying(&s.ty), "1", "", f, &mut held);
            }
            let before = d.relations.len();
            for (to, mult, key) in held {
                if let Some(to) = class_id(to) {
                    let label = format!("{}{key}", s.info.name);
                    d.relations.push(Relation { from: id.clone(), to, kind: RelKind::Aggregation, label, mult });
                }
            }
            if d.relations.len() == before {
                let init = s.init.as_ref().map(|e| format!(" = {}", f.src(e.span))).unwrap_or_default();
                c.attributes.push(format!("{}: {}{init}", s.info.name, f.ty_text(&s.ty)));
            }
        }
        for decl in f.decls.iter().filter(|x| x.module == module) {
            let DeclRef::Rule(rid) = decl.r else { continue };
            let r = p.rule(rid);
            let result = r.result.as_ref().map(|t| format!(" {}", f.ty_text(t))).unwrap_or_default();
            let sign = if decl.is_pub { '+' } else { '-' };
            c.operations.push(format!("{sign}{}({}){result}", decl.name, params_text(&r.params, f)));
        }
        d.classes.push(c);
    }
    d
}

impl ClassDiagram {
    /// Keep one class and every class one relation away from it.
    pub fn around(&self, id: &str) -> ClassDiagram {
        let relations: Vec<Relation> = self.relations.iter().filter(|r| r.from == id || r.to == id).cloned().collect();
        let keep = |c: &&Class| c.id == id || relations.iter().any(|r| r.from == c.id || r.to == c.id);
        ClassDiagram { classes: self.classes.iter().filter(keep).cloned().collect(), relations }
    }

    pub fn to_mermaid(&self) -> String {
        let attr = |s: &str| text::mermaid(s).replace('(', "#40;").replace(')', "#41;");
        let mut out = String::from("classDiagram\n");
        for c in &self.classes {
            let _ = writeln!(out, "  class {}[\"{}\"] {{", c.id, text::mermaid(&c.name));
            let _ = writeln!(out, "    <<{}>>", c.stereotype);
            for a in &c.attributes {
                let _ = writeln!(out, "    {}", attr(a));
            }
            for o in &c.operations {
                let _ = writeln!(out, "    {}", text::mermaid(o));
            }
            out.push_str("  }\n");
        }
        for c in &self.classes {
            for k in &c.constraints {
                let _ = writeln!(out, "  note for {} \"{{ {} }}\"", c.id, text::mermaid(k));
            }
        }
        for r in &self.relations {
            let arrow = match r.kind {
                RelKind::Composition => "*--",
                RelKind::Aggregation => "o--",
                RelKind::Alias => "..>",
            };
            let _ = writeln!(out, "  {} \"1\" {arrow} \"{}\" {} : {}", r.from, r.mult, r.to, text::mermaid(&r.label));
        }
        out
    }

    pub fn to_plantuml(&self) -> String {
        let mut out = String::from("@startuml\nhide empty members\n");
        for c in &self.classes {
            let _ = writeln!(out, "class \"{}\" as {} <<{}>> {{", text::plantuml(&c.name), c.id, c.stereotype);
            for a in &c.attributes {
                let _ = writeln!(out, "  {{field}} {}", text::plantuml(a));
            }
            for o in &c.operations {
                let _ = writeln!(out, "  {{method}} {}", text::plantuml(o));
            }
            out.push_str("}\n");
        }
        for c in &self.classes {
            for k in &c.constraints {
                let _ = writeln!(out, "note right of {} : {{ {} }}", c.id, text::plantuml(k));
            }
        }
        for r in &self.relations {
            let arrow = match r.kind {
                RelKind::Composition => "*--",
                RelKind::Aggregation => "o--",
                RelKind::Alias => "..>",
            };
            let _ = writeln!(out, "{} \"1\" {arrow} \"{}\" {} : {}", r.from, r.mult, r.to, text::plantuml(&r.label));
        }
        out.push_str("@enduml\n");
        out
    }

    /// Nodes are classes, edges are relations.
    pub fn to_json(&self) -> Json {
        let classes = self.classes.iter().map(|c| {
            Json::obj([
                ("id", Json::str(&c.id)),
                ("kind", Json::str(&c.stereotype)),
                ("label", Json::str(&c.name)),
                ("decl", Json::str(&c.decl)),
                ("attributes", Json::strs(&c.attributes)),
                ("operations", Json::strs(&c.operations)),
                ("constraints", Json::strs(&c.constraints)),
            ])
        });
        let relations = self.relations.iter().map(|r| {
            Json::obj([
                ("from", Json::str(&r.from)),
                ("to", Json::str(&r.to)),
                ("kind", Json::str(r.kind.as_str())),
                ("label", Json::str(&r.label)),
                ("multiplicity", Json::str(&r.mult)),
            ])
        });
        Json::obj([
            ("view", Json::str("class")),
            (
                "counts",
                Json::obj([("nodes", Json::count(self.classes.len())), ("edges", Json::count(self.relations.len()))]),
            ),
            ("nodes", Json::Arr(classes.collect())),
            ("edges", Json::Arr(relations.collect())),
        ])
    }
}
