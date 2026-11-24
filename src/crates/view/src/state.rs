//! `diagram --kind state`: one machine per sum-typed place.
//!
//! A machine comes from two sources. A `flow` gives its `@edge`s as written
//! ([`FlowEdge`], passed in as data: the IR has no `flow` yet). And rules are
//! read: an assignment `place = Variant` whose place has a sum type is a move
//! to that variant, and a `@guard` of the form `place == Variant` (or
//! `place.is(Variant)`) on the same place is where it moves from. With no such
//! guard the move is from any state. Constructing the record that holds the
//! place enters the machine; removing it from a state leaves the machine.
//!
//! Edges are labelled `rule [guard, ...] / effect`.

use crate::facts::{Facts, Kind, Phase, RefKind, Target, each_stmt, stmt_exprs, walk};
use crate::json::Json;
use crate::text;
use spec_lang_ir::{
    Callee, CmpOp, DeclRef, Expr, ExprKind, MutOp, Pattern, PlaceRoot, PlaceStep, RuleDecl, StmtKind, Ty, TypeDef,
    TypeId,
};
use std::fmt::Write as _;

/// One `@edge(from -> to, via: rule, by: who, after: t)` of a `flow`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FlowEdge {
    /// The flow's name.
    pub flow: String,
    /// The place it governs, as written: `Invite.status`.
    pub at: String,
    pub from: String,
    pub to: String,
    pub via: Option<String>,
    pub by: Option<String>,
    pub after: Option<String>,
}

/// An end of a transition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum End {
    /// Entering (as a source) or leaving (as a target) the machine.
    Outside,
    /// Any state of the machine: the source of an exit.
    Any,
    State(String),
    /// A value the rule computes; the IR does not say which variant.
    Computed,
}

impl End {
    fn text(&self) -> &str {
        match self {
            End::Outside => "[*]",
            End::Any => "(any)",
            End::State(s) => s,
            End::Computed => "(computed)",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transition {
    pub from: End,
    pub to: End,
    /// The rule (qualified), or the flow for an edge with no `via`.
    pub rule: String,
    /// What is printed for the rule.
    pub rule_label: String,
    /// The other guards of the rule, each `cond else Error`'s condition.
    pub guards: Vec<String>,
    /// Outside effects of the rule: `out.line`.
    pub effects: Vec<String>,
    /// `rule` when read off a rule body, `flow` when declared by `@edge`.
    pub source: String,
}

impl Transition {
    /// `rule [guard, guard] / effect`.
    pub fn label(&self) -> String {
        let mut s = self.rule_label.clone();
        if !self.guards.is_empty() {
            let _ = write!(s, " [{}]", self.guards.join(", "));
        }
        if !self.effects.is_empty() {
            let _ = write!(s, " / {}", self.effects.join(", "));
        }
        s
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Machine {
    /// `todo.model.Task.status`, or the flow's `at`.
    pub name: String,
    /// The sum type's name; empty for a flow whose type the IR does not give.
    pub ty: String,
    /// Every variant, moved to or not.
    pub states: Vec<String>,
    pub transitions: Vec<Transition>,
}

/// Where a sum-typed value lives: a field of a record, or a state (or its entries).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Site {
    Field(TypeId, String),
    State(usize),
}

struct Reader<'a, 'p> {
    f: &'a Facts<'p>,
    machines: Vec<(Site, TypeId, Machine)>,
}

impl Reader<'_, '_> {
    fn machine(&mut self, site: Site, sum: TypeId) -> &mut Machine {
        let at = match self.machines.iter().position(|(s, _, _)| *s == site) {
            Some(i) => i,
            None => {
                let f = self.f;
                let name = match &site {
                    Site::Field(rec, field) => match f.index_of(DeclRef::Type(*rec)) {
                        Some(i) => format!("{}.{field}", f.decls[i].qname),
                        None => field.clone(),
                    },
                    Site::State(i) => f.decls[*i].qname.clone(),
                };
                let states = match &f.p.ty(sum).def {
                    TypeDef::Sum(vs) => vs.iter().map(|v| v.name.clone()).collect(),
                    _ => Vec::new(),
                };
                let ty = f.short_of(DeclRef::Type(sum));
                self.machines.push((site, sum, Machine { name, ty, states, transitions: Vec::new() }));
                self.machines.len() - 1
            }
        };
        &mut self.machines[at].2
    }
}

/// The variant an expression is, when it is written as one.
fn variant_of(e: &Expr, sum: TypeId) -> Option<u32> {
    match &e.kind {
        ExprKind::Variant { ty, tag } if *ty == sum => Some(*tag),
        ExprKind::Call { callee: Callee::Variant(ty, tag), .. } if *ty == sum => Some(*tag),
        _ => None,
    }
}

/// The guard that pins `place` to one variant, as (guard index, tag).
fn source_guard(rule: &RuleDecl, place: &str, sum: TypeId, f: &Facts<'_>) -> Option<(usize, u32)> {
    rule.guards.iter().enumerate().find_map(|(i, g)| match &g.cond.kind {
        ExprKind::Compare(lhs, rest) if f.src(lhs.span) == place => match rest.as_slice() {
            [(CmpOp::Eq, rhs)] => variant_of(rhs, sum).map(|t| (i, t)),
            _ => None,
        },
        ExprKind::Is { value, pattern } if f.src(value.span) == place => match pattern.as_ref() {
            Pattern::Variant { ty, tag, .. } if *ty == sum => Some((i, *tag)),
            _ => None,
        },
        _ => None,
    })
}

/// Every record construction `T(...)` in an expression, with its arguments.
fn constructions<'a>(e: &'a Expr, f: &'a Facts<'_>, out: &mut Vec<(TypeId, &'a [Option<Expr>])>) {
    walk(f.p, e, &mut |x| {
        if let ExprKind::Call { callee: Callee::Record(id), args } = &x.kind {
            out.push((*id, args.as_slice()));
        }
    });
}

/// Every machine of a program: declared `flows` first, then those read off
/// rules. A sum-typed place that is only entered and left, never moved, is not
/// a machine; [`unmoved_places`] lists those.
pub fn state_machines(f: &Facts<'_>, flows: &[FlowEdge]) -> Vec<Machine> {
    let mut out = flow_machines(flows);
    out.extend(read_machines(f).into_iter().filter(Machine::moves));
    out
}

/// Sum-typed places that rules create or remove but never move between variants.
pub fn unmoved_places(f: &Facts<'_>) -> Vec<String> {
    read_machines(f).into_iter().filter(|m| !m.moves()).map(|m| m.name).collect()
}

fn read_machines(f: &Facts<'_>) -> Vec<Machine> {
    let p = f.p;
    let mut rd = Reader { f, machines: Vec::new() };

    // A sum-typed state starts where its initial value says.
    for (i, d) in f.decls.iter().enumerate() {
        let DeclRef::State(sid) = d.r else { continue };
        let s = p.state(sid);
        let (Some(sum), Some(init)) = (f.sum_of(&s.ty), &s.init) else { continue };
        if let Some(tag) = variant_of(init, sum) {
            let to = End::State(f.variant_name(sum, tag));
            rd.machine(Site::State(i), sum).transitions.push(Transition {
                from: End::Outside,
                to,
                rule: d.qname.clone(),
                rule_label: "initial".to_string(),
                guards: Vec::new(),
                effects: Vec::new(),
                source: "state".to_string(),
            });
        }
    }

    for (ri, d) in f.decls.iter().enumerate() {
        let DeclRef::Rule(rid) = d.r else { continue };
        let rule = p.rule(rid);
        let effects: Vec<String> = if f.touch(ri).out { vec!["out.line".to_string()] } else { Vec::new() };
        let all_guards: Vec<String> = rule.guards.iter().map(|g| f.src(g.cond.span)).collect();
        let step = |from: End, to: End, skip: Option<usize>| Transition {
            from,
            to,
            rule: d.qname.clone(),
            rule_label: f.short(ri).to_string(),
            guards: all_guards.iter().enumerate().filter(|(i, _)| Some(*i) != skip).map(|(_, g)| g.clone()).collect(),
            effects: effects.clone(),
            source: "rule".to_string(),
        };
        let ExprKind::Block(body) = &rule.body.kind else { continue };
        let mut stmts = Vec::new();
        each_stmt(body, &mut stmts);
        for s in stmts {
            // Entering: a record with a sum-typed field is constructed.
            let mut exprs = Vec::new();
            stmt_exprs(s, &mut exprs);
            let mut made = Vec::new();
            exprs.iter().for_each(|e| constructions(e, f, &mut made));
            for (rec, args) in made {
                let TypeDef::Record(fields) = &p.ty(rec).def else { continue };
                for (n, field) in fields.iter().enumerate() {
                    let Some(sum) = f.sum_of(&field.ty) else { continue };
                    let value = args.get(n).and_then(Option::as_ref).or(field.default.as_ref());
                    let to = match value.and_then(|v| variant_of(v, sum)) {
                        Some(tag) => End::State(f.variant_name(sum, tag)),
                        None => End::Computed,
                    };
                    let t = step(End::Outside, to, None);
                    let m = rd.machine(Site::Field(rec, field.name.clone()), sum);
                    if !m.transitions.contains(&t) {
                        m.transitions.push(t);
                    }
                }
            }
            match &s.kind {
                // Moving: `place = Variant`.
                StmtKind::Assign { place, value } => {
                    let Some(sum) = f.place_ty(place).and_then(|t| f.sum_of(t)) else { continue };
                    let PlaceRoot::State(sid) = &place.root else { continue };
                    let site = match place.path.last() {
                        Some(PlaceStep::Field(name)) => {
                            let mut owner = place.clone();
                            owner.path.pop();
                            match f.place_ty(&owner).and_then(|t| f.record_of(t)) {
                                Some(rec) => Site::Field(rec, name.clone()),
                                None => continue,
                            }
                        }
                        _ => match f.index_of(DeclRef::State(*sid)) {
                            Some(i) => Site::State(i),
                            None => continue,
                        },
                    };
                    let to = match variant_of(value, sum) {
                        Some(tag) => End::State(f.variant_name(sum, tag)),
                        None => End::Computed,
                    };
                    let pinned = source_guard(rule, &f.src(place.span), sum, f);
                    // With no guard that pins the source, the move is from every state.
                    let froms: Vec<u32> = match (pinned, &f.p.ty(sum).def) {
                        (Some((_, tag)), _) => vec![tag],
                        (None, TypeDef::Sum(vs)) => (0u32..).take(vs.len()).collect(),
                        (None, _) => Vec::new(),
                    };
                    for tag in froms {
                        let t = step(End::State(f.variant_name(sum, tag)), to.clone(), pinned.map(|(i, _)| i));
                        let m = rd.machine(site.clone(), sum);
                        if !m.transitions.contains(&t) {
                            m.transitions.push(t);
                        }
                    }
                }
                // Leaving: an entry that holds a machine is removed from a state.
                StmtKind::Mutate { place, op: MutOp::Remove, .. } => {
                    let Some(ty) = f.place_ty(place) else { continue };
                    let held = match f.underlying(ty) {
                        Ty::Map(_, v) => v.as_ref(),
                        Ty::Set(t) | Ty::List(t) => t.as_ref(),
                        _ => continue,
                    };
                    if let Some(rec) = f.record_of(held) {
                        let TypeDef::Record(fields) = &p.ty(rec).def else { continue };
                        for field in fields {
                            let Some(sum) = f.sum_of(&field.ty) else { continue };
                            let t = step(End::Any, End::Outside, None);
                            let m = rd.machine(Site::Field(rec, field.name.clone()), sum);
                            if !m.transitions.contains(&t) {
                                m.transitions.push(t);
                            }
                        }
                    } else if let (Some(sum), PlaceRoot::State(sid)) = (f.sum_of(held), &place.root) {
                        let Some(i) = f.index_of(DeclRef::State(*sid)) else { continue };
                        let t = step(End::Any, End::Outside, None);
                        let m = rd.machine(Site::State(i), sum);
                        if !m.transitions.contains(&t) {
                            m.transitions.push(t);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    rd.machines.into_iter().map(|(_, _, m)| m).collect()
}

/// One machine per flow, its states in the order the edges name them.
fn flow_machines(flows: &[FlowEdge]) -> Vec<Machine> {
    let mut out: Vec<Machine> = Vec::new();
    for e in flows {
        let at = match out.iter().position(|m| m.name == e.at) {
            Some(i) => i,
            None => {
                out.push(Machine {
                    name: e.at.clone(),
                    ty: String::new(),
                    states: Vec::new(),
                    transitions: Vec::new(),
                });
                out.len() - 1
            }
        };
        let m = &mut out[at];
        for s in [&e.from, &e.to] {
            if !m.states.contains(s) {
                m.states.push(s.clone());
            }
        }
        let mut guards = Vec::new();
        if let Some(by) = &e.by {
            guards.push(format!("by {by}"));
        }
        if let Some(after) = &e.after {
            guards.push(format!("after {after}"));
        }
        m.transitions.push(Transition {
            from: End::State(e.from.clone()),
            to: End::State(e.to.clone()),
            rule: e.via.clone().unwrap_or_else(|| e.flow.clone()),
            rule_label: e.via.clone().unwrap_or_else(|| e.flow.clone()),
            guards,
            effects: Vec::new(),
            source: "flow".to_string(),
        });
    }
    out
}

/// The rules that write a state but move no machine of it: listed so a reader
/// sees which writers the diagram does not show.
pub fn silent_writers(f: &Facts<'_>, machines: &[Machine]) -> Vec<String> {
    let mut out = Vec::new();
    for d in f.decls.iter().filter(|d| d.kind == Kind::Rule) {
        let writes = d
            .refs
            .iter()
            .any(|r| r.kind == RefKind::Writes && r.phase == Phase::Body && matches!(r.to, Target::Decl(_)));
        let moves = machines.iter().any(|m| m.transitions.iter().any(|t| t.rule == d.qname));
        if writes && !moves {
            out.push(d.qname.clone());
        }
    }
    out
}

impl Machine {
    /// Has a transition that starts inside the machine and ends inside it.
    fn moves(&self) -> bool {
        self.transitions.iter().any(|t| t.from != End::Outside && t.to != End::Outside)
    }

    fn ids(&self) -> (String, Vec<(String, String)>) {
        let mut ids = text::Ids::new();
        let box_id = ids.fresh("machine");
        let mut states: Vec<(String, String)> = self.states.iter().map(|s| (s.clone(), ids.fresh(s))).collect();
        if self.transitions.iter().any(|t| t.to == End::Computed) {
            states.push(("(computed)".to_string(), ids.fresh("computed")));
        }
        (box_id, states)
    }

    /// Nodes: the states, plus one for entry and exit if any transition uses it. Edges: the transitions.
    pub fn node_count(&self) -> usize {
        let outside = self.transitions.iter().any(|t| t.from == End::Outside || t.to == End::Outside);
        self.ids().1.len() + usize::from(outside)
    }

    fn end_id<'a>(end: &End, box_id: &'a str, states: &'a [(String, String)]) -> &'a str {
        match end {
            End::Outside => "[*]",
            End::Any => box_id,
            End::State(s) => states.iter().find(|(n, _)| n == s).map_or("[*]", |(_, id)| id.as_str()),
            End::Computed => states.last().map_or("[*]", |(_, id)| id.as_str()),
        }
    }

    /// A composite state holding the variants. A move from any state leaves
    /// from the composite's border; entry and exit are `[*]`.
    pub fn to_mermaid(&self) -> String {
        let (box_id, states) = self.ids();
        let title = if self.ty.is_empty() { self.name.clone() } else { format!("{} : {}", self.name, self.ty) };
        let mut out = String::from("stateDiagram-v2\n");
        let _ = writeln!(out, "  state \"{}\" as {box_id} {{", text::mermaid(&title).replace(':', "#58;"));
        for (name, id) in &states {
            let _ = writeln!(out, "    state \"{}\" as {id}", text::mermaid(name));
        }
        let inside = |t: &&Transition| t.from != End::Any && t.to != End::Outside;
        for t in self.transitions.iter().filter(inside) {
            let (a, b) = (Self::end_id(&t.from, &box_id, &states), Self::end_id(&t.to, &box_id, &states));
            let _ = writeln!(out, "    {a} --> {b} : {}", text::mermaid(&t.label()).replace(':', "#58;"));
        }
        out.push_str("  }\n");
        for t in self.transitions.iter().filter(|t| !inside(t)) {
            let (a, b) = (Self::end_id(&t.from, &box_id, &states), Self::end_id(&t.to, &box_id, &states));
            let _ = writeln!(out, "  {a} --> {b} : {}", text::mermaid(&t.label()).replace(':', "#58;"));
        }
        out
    }

    pub fn to_plantuml(&self) -> String {
        let (box_id, states) = self.ids();
        let title = if self.ty.is_empty() { self.name.clone() } else { format!("{} : {}", self.name, self.ty) };
        let mut out = String::from("@startuml\n");
        let _ = writeln!(out, "state \"{}\" as {box_id} {{", text::plantuml(&title));
        for (name, id) in &states {
            let _ = writeln!(out, "  state \"{}\" as {id}", text::plantuml(name));
        }
        let inside = |t: &&Transition| t.from != End::Any && t.to != End::Outside;
        for t in self.transitions.iter().filter(inside) {
            let (a, b) = (Self::end_id(&t.from, &box_id, &states), Self::end_id(&t.to, &box_id, &states));
            let _ = writeln!(out, "  {a} --> {b} : {}", text::plantuml(&t.label()));
        }
        out.push_str("}\n");
        for t in self.transitions.iter().filter(|t| !inside(t)) {
            let (a, b) = (Self::end_id(&t.from, &box_id, &states), Self::end_id(&t.to, &box_id, &states));
            let _ = writeln!(out, "{a} --> {b} : {}", text::plantuml(&t.label()));
        }
        out.push_str("@enduml\n");
        out
    }

    pub fn to_json(&self) -> Json {
        let edges = self.transitions.iter().map(|t| {
            Json::obj([
                ("from", Json::str(t.from.text())),
                ("to", Json::str(t.to.text())),
                ("kind", Json::str(&t.source)),
                ("rule", Json::str(&t.rule)),
                ("guards", Json::strs(&t.guards)),
                ("effects", Json::strs(&t.effects)),
                ("label", Json::str(t.label())),
            ])
        });
        Json::obj([
            ("machine", Json::str(&self.name)),
            ("type", Json::str(&self.ty)),
            (
                "counts",
                Json::obj([("nodes", Json::count(self.node_count())), ("edges", Json::count(self.transitions.len()))]),
            ),
            ("nodes", Json::strs(&self.states)),
            ("edges", Json::Arr(edges.collect())),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn invite() -> Vec<FlowEdge> {
        let edge = |from: &str, to: &str, via: Option<&str>, by: Option<&str>, after: Option<&str>| FlowEdge {
            flow: "InviteLife".to_string(),
            at: "Invite.status".to_string(),
            from: from.to_string(),
            to: to.to_string(),
            via: via.map(str::to_string),
            by: by.map(str::to_string),
            after: after.map(str::to_string),
        };
        vec![
            edge("Open", "Accepted", Some("accept"), None, None),
            edge("Open", "Revoked", Some("revoke"), Some("admin"), None),
            edge("Open", "Expired", None, None, Some("7d")),
        ]
    }

    #[test]
    fn flow_edges_make_one_machine() {
        let ms = flow_machines(&invite());
        assert_eq!(ms.len(), 1);
        assert_eq!(ms[0].states, ["Open", "Accepted", "Revoked", "Expired"]);
        assert_eq!((ms[0].node_count(), ms[0].transitions.len()), (4, 3));
        let want = "stateDiagram-v2
  state \"Invite.status\" as machine {
    state \"Open\" as Open
    state \"Accepted\" as Accepted
    state \"Revoked\" as Revoked
    state \"Expired\" as Expired
    Open --> Accepted : accept
    Open --> Revoked : revoke [by admin]
    Open --> Expired : InviteLife [after 7d]
  }
";
        assert_eq!(ms[0].to_mermaid(), want);
    }

    #[test]
    fn flow_machine_prints_plantuml() {
        let ms = flow_machines(&invite());
        let want = "@startuml
state \"Invite.status\" as machine {
  state \"Open\" as Open
  state \"Accepted\" as Accepted
  state \"Revoked\" as Revoked
  state \"Expired\" as Expired
  Open --> Accepted : accept
  Open --> Revoked : revoke [by admin]
  Open --> Expired : InviteLife [after 7d]
}
@enduml
";
        assert_eq!(ms[0].to_plantuml(), want);
    }
}
