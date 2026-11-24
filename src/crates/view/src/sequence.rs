//! `diagram --kind sequence`: who calls what, in order.
//!
//! [`test_sequence`] reads a `test` without running it: each rule call in the
//! body is a message from the test to the rule's module, followed by the state
//! that rule can read and write and the outside effects it can request (its
//! static sets, see [`crate::facts::Facts::touch`]), then the reply the test
//! expects. A call the test expects to be refused shows only the reads of the
//! rule's guards: a refused rule runs nothing else.
//!
//! [`trace_sequence`] draws a trace that was run elsewhere (a law's
//! counterexample): the engine passes the steps as data.

use crate::facts::{Facts, stmt_exprs, walk};
use crate::json::Json;
use crate::text;
use spec_lang_ir::{Block, Callee, DeclRef, Expect, Expr, ExprKind, Given, RuleId, StmtKind, TestId};
use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sequence {
    pub title: String,
    pub participants: Vec<Participant>,
    pub items: Vec<Item>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    /// The test, or whoever drove the trace.
    Actor,
    /// A module's rules.
    Rules,
    State,
    /// An outside effect handler (`out`).
    Outside,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Actor => "actor",
            Role::Rules => "rules",
            Role::State => "state",
            Role::Outside => "outside",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Participant {
    pub id: String,
    pub label: String,
    pub role: Role,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arrow {
    Call,
    Reply,
    /// A reply that is a refusal: `fail E`.
    Refusal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Frame {
    /// `if` with no `else`.
    Opt,
    /// `if` with an `else`.
    Alt,
    /// `for` or `while`.
    Loop,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Item {
    Message {
        from: String,
        to: String,
        text: String,
        arrow: Arrow,
    },
    /// A note over one lane, or spanning from the first to the last of several.
    Note {
        over: Vec<String>,
        text: String,
    },
    Open {
        frame: Frame,
        label: String,
    },
    Else,
    Close,
}

/// Longest text of a note; a longer one is cut and marked with `…`.
const NOTE_MAX: usize = 96;

struct Builder {
    seq: Sequence,
    ids: text::Ids,
}

impl Builder {
    fn new(title: &str, actor: &str) -> Self {
        let mut b = Builder {
            seq: Sequence { title: title.to_string(), participants: Vec::new(), items: Vec::new() },
            ids: text::Ids::new(),
        };
        b.participant(actor, Role::Actor);
        b
    }

    /// The participant with this label and role, added on first use.
    fn participant(&mut self, label: &str, role: Role) -> String {
        if let Some(p) = self.seq.participants.iter().find(|p| p.label == label && p.role == role) {
            return p.id.clone();
        }
        let prefix = match role {
            // Not `actor`: that word is a keyword of both diagram languages.
            Role::Actor => "caller",
            Role::Rules => "m",
            Role::State => "s",
            Role::Outside => "x",
        };
        let id =
            if role == Role::Actor { self.ids.fresh(prefix) } else { self.ids.fresh(&format!("{prefix}_{label}")) };
        self.seq.participants.push(Participant { id: id.clone(), label: label.to_string(), role });
        id
    }

    fn actor(&self) -> String {
        self.seq.participants.first().map(|p| p.id.clone()).unwrap_or_default()
    }

    fn message(&mut self, from: &str, to: &str, text: &str, arrow: Arrow) {
        self.seq.items.push(Item::Message {
            from: from.to_string(),
            to: to.to_string(),
            text: text.to_string(),
            arrow,
        });
    }

    fn note(&mut self, over: &str, text: &str) {
        self.note_over(vec![over.to_string()], text);
    }

    fn note_over(&mut self, over: Vec<String>, text: &str) {
        self.seq.items.push(Item::Note { over, text: text::shorten(text, NOTE_MAX) });
    }

    /// Participants in lanes: the actor, rules, state, outside; first use first within a lane.
    fn finish(mut self) -> Sequence {
        self.seq.participants.sort_by_key(|p| p.role);
        self.seq
    }
}

/// What a test step expects of a rule call.
enum Reply<'a> {
    /// Called for its effect; any reply that is not a refusal.
    Any,
    Expect(&'a Expect),
}

impl Builder {
    /// One rule call: the call, what the rule can touch, the reply.
    fn rule_call(&mut self, f: &Facts<'_>, rule: RuleId, call: &Expr, reply: &Reply<'_>) {
        let Some(ri) = f.index_of(DeclRef::Rule(rule)) else { return };
        let actor = self.actor();
        let lane = self.participant(&f.decls[ri].module, Role::Rules);
        self.message(&actor, &lane, &f.src(call.span), Arrow::Call);
        let touch = f.touch(ri);
        let refused = matches!(reply, Reply::Expect(Expect::Fail(_)));
        let states: Vec<usize> = if refused {
            touch.guard_reads.iter().copied().collect()
        } else {
            touch.reads.union(&touch.writes).copied().collect()
        };
        for s in states {
            let mut verbs = Vec::new();
            if touch.reads.contains(&s) {
                verbs.push("read");
            }
            if !refused && touch.writes.contains(&s) {
                verbs.push("write");
            }
            let to = self.participant(f.short(s), Role::State);
            self.message(&lane, &to, &verbs.join(", "), Arrow::Call);
        }
        if touch.out && !refused {
            let out = self.participant("out", Role::Outside);
            self.message(&lane, &out, "line", Arrow::Call);
        }
        match reply {
            Reply::Any => self.message(&lane, &actor, "ok", Arrow::Reply),
            Reply::Expect(Expect::Value(v)) => self.message(&lane, &actor, &f.src(v.span), Arrow::Reply),
            Reply::Expect(Expect::Fail(e)) => self.message(&lane, &actor, &format!("fail {}", e.name), Arrow::Refusal),
        }
    }

    /// The states and handlers an expression names get a lane, so a note about them has a place.
    fn mention(&mut self, f: &Facts<'_>, e: &Expr) {
        walk(f.p, e, &mut |x| match &x.kind {
            ExprKind::State(id) => {
                if let Some(i) = f.index_of(DeclRef::State(*id)) {
                    self.participant(f.short(i), Role::State);
                }
            }
            ExprKind::EffectRead(e) => {
                let name = f.p.handler(e.handler).name.clone();
                self.participant(&name, Role::Outside);
            }
            _ => {}
        });
    }

    fn block(&mut self, f: &Facts<'_>, b: &Block) {
        let actor = self.actor();
        for s in &b.stmts {
            let stmt_text = f.src(s.span);
            let stmt_text = stmt_text.trim_end_matches(';');
            match &s.kind {
                StmtKind::Expect { subject, expect } => match &subject.kind {
                    ExprKind::Call { callee: Callee::Rule(r), .. } => {
                        self.rule_call(f, *r, subject, &Reply::Expect(expect));
                    }
                    _ => {
                        self.calls_in(f, subject);
                        self.note(&actor, &format!("expect {stmt_text}"));
                    }
                },
                StmtKind::Expr(e) => match &e.kind {
                    ExprKind::Call { callee: Callee::Rule(r), .. } => self.rule_call(f, *r, e, &Reply::Any),
                    _ => {
                        self.calls_in(f, e);
                        self.note(&actor, stmt_text);
                    }
                },
                StmtKind::If { cond, then, els } => {
                    self.calls_in(f, cond);
                    let frame = if els.is_some() { Frame::Alt } else { Frame::Opt };
                    self.seq.items.push(Item::Open { frame, label: f.src(cond.span) });
                    self.block(f, then);
                    if let Some(els) = els {
                        self.seq.items.push(Item::Else);
                        self.block(f, els);
                    }
                    self.seq.items.push(Item::Close);
                }
                StmtKind::For { var, iter, body } => {
                    self.calls_in(f, iter);
                    self.seq
                        .items
                        .push(Item::Open { frame: Frame::Loop, label: format!("for {var} in {}", f.src(iter.span)) });
                    self.block(f, body);
                    self.seq.items.push(Item::Close);
                }
                StmtKind::While { cond, body } => {
                    self.seq
                        .items
                        .push(Item::Open { frame: Frame::Loop, label: format!("while {}", f.src(cond.span)) });
                    self.calls_in(f, cond);
                    self.block(f, body);
                    self.seq.items.push(Item::Close);
                }
                StmtKind::Let { .. }
                | StmtKind::Assign { .. }
                | StmtKind::Mutate { .. }
                | StmtKind::Return(_)
                | StmtKind::Assert(_) => {
                    let mut exprs = Vec::new();
                    stmt_exprs(s, &mut exprs);
                    for e in exprs {
                        self.calls_in(f, e);
                    }
                    self.note(&actor, stmt_text);
                }
            }
        }
    }

    /// Rule calls nested in an expression, each drawn with no expected reply.
    fn calls_in(&mut self, f: &Facts<'_>, e: &Expr) {
        let mut calls = Vec::new();
        walk(f.p, e, &mut |x| {
            if let ExprKind::Call { callee: Callee::Rule(r), .. } = &x.kind {
                calls.push((*r, x));
            }
        });
        for (r, x) in calls {
            self.rule_call(f, r, x, &Reply::Any);
        }
        self.mention(f, e);
    }
}

/// The sequence diagram of one test, read off its body.
pub fn test_sequence(f: &Facts<'_>, id: TestId) -> Sequence {
    let t = f.p.test(id);
    let i = f.index_of(DeclRef::Test(id));
    let title = i.map_or_else(|| t.info.name.clone(), |i| f.decls[i].qname.clone());
    let mut b = Builder::new(&title, &t.info.name);
    let actor = b.actor();
    if !t.params.is_empty() {
        b.note(&actor, &format!("for any {}", crate::class::params_text(&t.params, f)));
    }
    for g in &t.given {
        match g {
            Given::State { state, value, .. } => {
                let name = f.short_of(DeclRef::State(*state));
                let lane = b.participant(&name, Role::State);
                b.note(&lane, &format!("given {}", f.src(value.span)));
            }
            Given::Test { test, .. } => {
                // The states the given test leaves set, through its own givens.
                let mut lanes = Vec::new();
                let mut todo = vec![*test];
                let mut seen = Vec::new();
                while let Some(tid) = todo.pop() {
                    if seen.contains(&tid) {
                        continue;
                    }
                    seen.push(tid);
                    for g in &f.p.test(tid).given {
                        match g {
                            Given::State { state, .. } => lanes.push(f.short_of(DeclRef::State(*state))),
                            Given::Test { test, .. } => todo.push(*test),
                        }
                    }
                }
                let text = format!("given {}", f.short_of(DeclRef::Test(*test)));
                let over: Vec<String> = lanes.iter().map(|name| b.participant(name, Role::State)).collect();
                if over.is_empty() {
                    b.note(&actor, &text);
                } else {
                    b.note_over(over, &text);
                }
            }
        }
    }
    if !t.cases.is_empty() {
        b.note(&actor, &format!("{} @cases rows", t.cases.len()));
    }
    b.block(f, &t.body);
    b.finish()
}

/// Every test of the program, in source order.
pub fn test_sequences(f: &Facts<'_>) -> Vec<Sequence> {
    f.decls
        .iter()
        .filter_map(|d| match d.r {
            DeclRef::Test(id) => Some(test_sequence(f, id)),
            _ => None,
        })
        .collect()
}

/// The reply of one step of a trace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceReply {
    /// Committed, with no reply value.
    Ok,
    /// Committed, with this reply (printed by the caller).
    Value(String),
    /// Refused with this error: nothing was written.
    Fail(String),
}

/// One step of a trace that was run: a rule call and what it did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceStep {
    /// The module of the rule: `todo.board`.
    pub module: String,
    /// The call as text: `done(2)`.
    pub call: String,
    pub reply: TraceReply,
    /// States read, by name.
    pub reads: Vec<String>,
    /// States written: name, and the change as text (may be empty).
    pub writes: Vec<(String, String)>,
    /// Lines sent to `out`.
    pub out: Vec<String>,
}

/// A trace to draw: a law's counterexample, or any recorded run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Trace {
    /// What the diagram is called: the law's qualified name.
    pub title: String,
    /// Who drives it: `trace`, or the law's name.
    pub actor: String,
    /// The start world: state name and its value as text.
    pub given: Vec<(String, String)>,
    pub steps: Vec<TraceStep>,
    /// What the trace shows, printed last: `refutes numbers_below_next at step 3`.
    pub verdict: Option<String>,
}

/// The sequence diagram of a trace. Pure: the steps are data, nothing is run.
pub fn trace_sequence(t: &Trace) -> Sequence {
    let mut b = Builder::new(&t.title, &t.actor);
    let actor = b.actor();
    for (state, value) in &t.given {
        let lane = b.participant(state, Role::State);
        b.note(&lane, &format!("given {value}"));
    }
    for step in &t.steps {
        let lane = b.participant(&step.module, Role::Rules);
        b.message(&actor, &lane, &step.call, Arrow::Call);
        for state in &step.reads {
            let to = b.participant(state, Role::State);
            b.message(&lane, &to, "read", Arrow::Call);
        }
        for (state, change) in &step.writes {
            let to = b.participant(state, Role::State);
            let text = if change.is_empty() { "write".to_string() } else { format!("write {change}") };
            b.message(&lane, &to, &text, Arrow::Call);
        }
        for line in &step.out {
            let out = b.participant("out", Role::Outside);
            b.message(&lane, &out, &format!("line {line:?}"), Arrow::Call);
        }
        match &step.reply {
            TraceReply::Ok => b.message(&lane, &actor, "ok", Arrow::Reply),
            TraceReply::Value(v) => b.message(&lane, &actor, v, Arrow::Reply),
            TraceReply::Fail(e) => b.message(&lane, &actor, &format!("fail {e}"), Arrow::Refusal),
        }
    }
    if let Some(v) = &t.verdict {
        b.note(&actor, v);
    }
    b.finish()
}

/// The first and last lane of a note: both diagram languages span between two names.
fn span(over: &[String]) -> Vec<&str> {
    match over {
        [first, .., last] if first != last => vec![first, last],
        [first, ..] => vec![first],
        [] => Vec::new(),
    }
}

impl Sequence {
    pub fn message_count(&self) -> usize {
        self.items.iter().filter(|i| matches!(i, Item::Message { .. })).count()
    }

    pub fn to_mermaid(&self) -> String {
        let mut out = String::from("sequenceDiagram\n");
        for p in &self.participants {
            let word = if p.role == Role::Actor { "actor" } else { "participant" };
            let _ = writeln!(out, "  {word} {} as {}", p.id, text::mermaid(&p.label));
        }
        let mut depth = 1;
        for item in &self.items {
            let pad = "  ".repeat(depth);
            match item {
                Item::Message { from, to, text, arrow } => {
                    let a = match arrow {
                        Arrow::Call => "->>",
                        Arrow::Reply => "-->>",
                        Arrow::Refusal => "--x",
                    };
                    let _ = writeln!(out, "{pad}{from}{a}{to}: {}", text::mermaid(text));
                }
                Item::Note { over, text } => {
                    let _ = writeln!(out, "{pad}Note over {}: {}", span(over).join(","), text::mermaid(text));
                }
                Item::Open { frame, label } => {
                    let word = match frame {
                        Frame::Opt => "opt",
                        Frame::Alt => "alt",
                        Frame::Loop => "loop",
                    };
                    let _ = writeln!(out, "{pad}{word} {}", text::mermaid(label));
                    depth += 1;
                }
                Item::Else => {
                    let _ = writeln!(out, "{}else", "  ".repeat(depth.saturating_sub(1)));
                }
                Item::Close => {
                    depth = depth.saturating_sub(1).max(1);
                    let _ = writeln!(out, "{}end", "  ".repeat(depth));
                }
            }
        }
        out
    }

    pub fn to_plantuml(&self) -> String {
        let mut out = String::from("@startuml\n");
        let _ = writeln!(out, "title {}", text::plantuml(&self.title));
        for p in &self.participants {
            let word = match p.role {
                Role::Actor => "actor",
                Role::Rules => "participant",
                Role::State => "database",
                Role::Outside => "boundary",
            };
            let _ = writeln!(out, "{word} \"{}\" as {}", text::plantuml(&p.label), p.id);
        }
        for item in &self.items {
            match item {
                Item::Message { from, to, text, arrow } => {
                    let a = match arrow {
                        Arrow::Call => "->",
                        Arrow::Reply => "-->",
                        Arrow::Refusal => "--x",
                    };
                    let _ = writeln!(out, "{from} {a} {to} : {}", text::plantuml(text));
                }
                Item::Note { over, text } => {
                    let _ = writeln!(out, "note over {} : {}", span(over).join(", "), text::plantuml(text));
                }
                Item::Open { frame, label } => {
                    let word = match frame {
                        Frame::Opt => "opt",
                        Frame::Alt => "alt",
                        Frame::Loop => "loop",
                    };
                    let _ = writeln!(out, "{word} {}", text::plantuml(label));
                }
                Item::Else => out.push_str("else\n"),
                Item::Close => out.push_str("end\n"),
            }
        }
        out.push_str("@enduml\n");
        out
    }

    /// Nodes are participants, edges are messages in order; notes and frames are `steps`.
    pub fn to_json(&self) -> Json {
        let nodes = self.participants.iter().map(|p| {
            Json::obj([("id", Json::str(&p.id)), ("kind", Json::str(p.role.as_str())), ("label", Json::str(&p.label))])
        });
        let steps = self.items.iter().map(|i| match i {
            Item::Message { from, to, text, arrow } => {
                let kind = match arrow {
                    Arrow::Call => "call",
                    Arrow::Reply => "reply",
                    Arrow::Refusal => "refusal",
                };
                Json::obj([
                    ("step", Json::str("message")),
                    ("from", Json::str(from)),
                    ("to", Json::str(to)),
                    ("kind", Json::str(kind)),
                    ("text", Json::str(text)),
                ])
            }
            Item::Note { over, text } => {
                Json::obj([("step", Json::str("note")), ("over", Json::strs(over)), ("text", Json::str(text))])
            }
            Item::Open { frame, label } => {
                let kind = match frame {
                    Frame::Opt => "opt",
                    Frame::Alt => "alt",
                    Frame::Loop => "loop",
                };
                Json::obj([("step", Json::str("open")), ("kind", Json::str(kind)), ("text", Json::str(label))])
            }
            Item::Else => Json::obj([("step", Json::str("else"))]),
            Item::Close => Json::obj([("step", Json::str("close"))]),
        });
        Json::obj([
            ("sequence", Json::str(&self.title)),
            (
                "counts",
                Json::obj([
                    ("nodes", Json::count(self.participants.len())),
                    ("edges", Json::count(self.message_count())),
                ]),
            ),
            ("nodes", Json::Arr(nodes.collect())),
            ("steps", Json::Arr(steps.collect())),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counterexample() -> Trace {
        Trace {
            title: "bank.laws.never_negative".to_string(),
            actor: "trace".to_string(),
            given: vec![("balance".to_string(), "5".to_string())],
            steps: vec![
                TraceStep {
                    module: "bank".to_string(),
                    call: "pay(3)".to_string(),
                    reply: TraceReply::Value("2".to_string()),
                    reads: vec!["balance".to_string()],
                    writes: vec![("balance".to_string(), "5 -> 2".to_string())],
                    out: vec!["paid 3".to_string()],
                },
                TraceStep {
                    module: "bank".to_string(),
                    call: "pay(9)".to_string(),
                    reply: TraceReply::Fail("Short".to_string()),
                    reads: vec!["balance".to_string()],
                    writes: vec![],
                    out: vec![],
                },
            ],
            verdict: Some("refutes never_negative at step 2; seed 7".to_string()),
        }
    }

    #[test]
    fn trace_prints_mermaid() {
        let want = "sequenceDiagram
  actor caller as trace
  participant m_bank as bank
  participant s_balance as balance
  participant x_out as out
  Note over s_balance: given 5
  caller->>m_bank: pay(3)
  m_bank->>s_balance: read
  m_bank->>s_balance: write 5 -#gt; 2
  m_bank->>x_out: line #quot;paid 3#quot;
  m_bank-->>caller: 2
  caller->>m_bank: pay(9)
  m_bank->>s_balance: read
  m_bank--xcaller: fail Short
  Note over caller: refutes never_negative at step 2#59; seed 7
";
        let s = trace_sequence(&counterexample());
        assert_eq!(s.to_mermaid(), want);
        assert_eq!((s.participants.len(), s.message_count()), (4, 8));
    }

    #[test]
    fn trace_prints_plantuml() {
        let want = "@startuml
title bank.laws.never_negative
actor \"trace\" as caller
participant \"bank\" as m_bank
database \"balance\" as s_balance
boundary \"out\" as x_out
note over s_balance : given 5
caller -> m_bank : pay(3)
m_bank -> s_balance : read
m_bank -> s_balance : write 5 -> 2
m_bank -> x_out : line <U+0022>paid 3<U+0022>
m_bank --> caller : 2
caller -> m_bank : pay(9)
m_bank -> s_balance : read
m_bank --x caller : fail Short
note over caller : refutes never_negative at step 2; seed 7
@enduml
";
        assert_eq!(trace_sequence(&counterexample()).to_plantuml(), want);
    }
}
