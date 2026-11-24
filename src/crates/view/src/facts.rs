//! What every projection reads off the IR: the declarations by qualified
//! name, and for each one what it calls, reads, writes, constructs and names.
//!
//! Nothing here runs a rule. A read or write is one that is written in the
//! body (or in a `fn` or `rule` the body calls), so the sets are what the
//! declaration can touch, not what one run does touch.

use spec_lang_ir::{
    Arm, Block, Callee, DeclInfo, DeclRef, Effect, Expect, Expr, ExprKind, Given, InterpPart, LambdaBody, PatArg,
    Pattern, Place, PlaceRoot, PlaceStep, Program, Stmt, StmtKind, Ty, TypeDef, TypeId,
};
use spec_lang_syntax::Span;
use std::collections::{BTreeSet, HashMap};

use crate::text;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Type,
    Const,
    State,
    Fn,
    Rule,
    Test,
    Law,
    App,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Type => "type",
            Kind::Const => "const",
            Kind::State => "state",
            Kind::Fn => "fn",
            Kind::Rule => "rule",
            Kind::Test => "test",
            Kind::Law => "law",
            Kind::App => "app",
        }
    }
}

/// How one declaration refers to another.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RefKind {
    Calls,
    Reads,
    Writes,
    Constructs,
    /// An outside effect is requested (`out.line`).
    Emits,
    /// Named in a signature, a field, a claim or a type body.
    Uses,
    /// Named in a pattern of `match` or `.is(...)`.
    Matches,
    /// A law or test ranges over traces of this rule.
    Traces,
    /// A test starts from this test's end world.
    Given,
    /// A test's `@given` sets this state.
    Sets,
    /// An app exposes this rule.
    Exposes,
}

impl RefKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RefKind::Calls => "calls",
            RefKind::Reads => "reads",
            RefKind::Writes => "writes",
            RefKind::Constructs => "constructs",
            RefKind::Emits => "emits",
            RefKind::Uses => "uses",
            RefKind::Matches => "matches",
            RefKind::Traces => "traces",
            RefKind::Given => "given",
            RefKind::Sets => "sets",
            RefKind::Exposes => "exposes",
        }
    }
}

/// Where in a declaration a reference is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    /// Parameters, result, fields, a `type` body, `@given`.
    Signature,
    /// A `@guard` clause: runs before the body, and is all a refused call runs.
    Guard,
    Body,
    /// A `@cases` row: a test of the declaration, not part of what it does.
    Cases,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    /// An index into [`Facts::decls`].
    Decl(usize),
    /// The `out` handler of the `cli` vocabulary: lines to the program's user.
    Out,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ref {
    pub kind: RefKind,
    pub to: Target,
    pub phase: Phase,
}

#[derive(Debug)]
pub struct Decl {
    pub r: DeclRef,
    pub kind: Kind,
    /// Dotted module name.
    pub module: String,
    pub name: String,
    /// `module.name`.
    pub qname: String,
    /// `qname` as an identifier, unique among declarations.
    pub id: String,
    pub is_pub: bool,
    pub why: Option<String>,
    pub span: Span,
    /// Every reference written in it, in source order, each once.
    pub refs: Vec<Ref>,
}

/// The state a declaration can touch, through everything it calls.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Touch {
    /// States read anywhere (indexes into [`Facts::decls`]).
    pub reads: BTreeSet<usize>,
    /// States read by `@guard` clauses: all a refused call reads.
    pub guard_reads: BTreeSet<usize>,
    pub writes: BTreeSet<usize>,
    /// Requests the outside effect `out.line`.
    pub out: bool,
}

#[derive(Debug)]
pub struct Facts<'p> {
    pub p: &'p Program,
    pub decls: Vec<Decl>,
    index: HashMap<DeclRef, usize>,
    unique: Vec<bool>,
}

/// The common part of any declaration.
pub fn info(p: &Program, r: DeclRef) -> &DeclInfo {
    match r {
        DeclRef::Type(id) => &p.ty(id).info,
        DeclRef::Const(id) => &p.constant(id).info,
        DeclRef::State(id) => &p.state(id).info,
        DeclRef::Fn(id) => &p.func(id).info,
        DeclRef::Rule(id) => &p.rule(id).info,
        DeclRef::Test(id) => &p.test(id).info,
        DeclRef::Law(id) => &p.law(id).info,
        DeclRef::App(id) => &p.app(id).info,
    }
}

fn kind_of(r: DeclRef) -> Kind {
    match r {
        DeclRef::Type(_) => Kind::Type,
        DeclRef::Const(_) => Kind::Const,
        DeclRef::State(_) => Kind::State,
        DeclRef::Fn(_) => Kind::Fn,
        DeclRef::Rule(_) => Kind::Rule,
        DeclRef::Test(_) => Kind::Test,
        DeclRef::Law(_) => Kind::Law,
        DeclRef::App(_) => Kind::App,
    }
}

impl<'p> Facts<'p> {
    /// Read every declaration of every module with a file, in module then source order.
    pub fn of(p: &'p Program) -> Self {
        let mut decls = Vec::new();
        let mut index = HashMap::new();
        let mut ids = text::Ids::new();
        for m in p.modules.iter().filter(|m| !m.path.is_empty()) {
            for &r in &m.decls {
                let i = info(p, r);
                let qname = format!("{}.{}", m.name, i.name);
                index.insert(r, decls.len());
                decls.push(Decl {
                    r,
                    kind: kind_of(r),
                    module: m.name.clone(),
                    name: i.name.clone(),
                    id: ids.fresh(&qname),
                    qname,
                    is_pub: i.is_pub,
                    why: i.why.clone(),
                    span: i.span,
                    refs: Vec::new(),
                });
            }
        }
        let unique = decls.iter().map(|d| decls.iter().filter(|o| o.name == d.name).count() == 1).collect();
        let mut facts = Facts { p, decls, index, unique };
        for i in 0..facts.decls.len() {
            let mut c = Collector { p, index: &facts.index, refs: Vec::new(), phase: Phase::Signature };
            c.decl(facts.decls[i].r);
            facts.decls[i].refs = c.refs;
        }
        facts
    }

    pub fn index_of(&self, r: DeclRef) -> Option<usize> {
        self.index.get(&r).copied()
    }

    /// The bare name where no other declaration has it, else the qualified name.
    pub fn short(&self, i: usize) -> &str {
        let d = &self.decls[i];
        if self.unique[i] { &d.name } else { &d.qname }
    }

    /// The bare name of the declaration `r`, or its qualified name when the bare one is shared.
    pub fn short_of(&self, r: DeclRef) -> String {
        match self.index_of(r) {
            Some(i) => self.short(i).to_string(),
            None => info(self.p, r).name.clone(),
        }
    }

    /// Module names with a file, in program order.
    pub fn modules(&self) -> Vec<&'p str> {
        self.p.modules.iter().filter(|m| !m.path.is_empty()).map(|m| m.name.as_str()).collect()
    }

    /// Find a declaration by qualified name, by a dotted suffix of it
    /// (`board.done`), or by bare name. `Err` lists the candidates when
    /// several match and is empty when none does.
    pub fn find(&self, name: &str) -> Result<usize, Vec<String>> {
        if let Some(i) = self.decls.iter().position(|d| d.qname == name) {
            return Ok(i);
        }
        let suffix = format!(".{name}");
        let hits: Vec<usize> = (0..self.decls.len()).filter(|&i| self.decls[i].qname.ends_with(&suffix)).collect();
        match hits.as_slice() {
            [one] => Ok(*one),
            many => Err(many.iter().map(|&i| self.decls[i].qname.clone()).collect()),
        }
    }

    /// `todo/model.slg:12`: the file by module name, so no local path is printed.
    pub fn location(&self, i: usize) -> String {
        let d = &self.decls[i];
        format!("{}:{}", module_file(&d.module), self.p.location(d.span).line)
    }

    /// A span's source on one line.
    pub fn src(&self, span: Span) -> String {
        text::one_line(self.p.snippet(span))
    }

    /// The state a declaration can touch, following calls of `fn`s and `rule`s.
    pub fn touch(&self, i: usize) -> Touch {
        let mut t = Touch::default();
        let mut seen = BTreeSet::new();
        self.touch_into(i, false, &mut seen, &mut t);
        let mut g = Touch::default();
        let mut seen = BTreeSet::new();
        self.touch_into(i, true, &mut seen, &mut g);
        t.guard_reads = g.reads;
        t
    }

    fn touch_into(&self, i: usize, guards_only: bool, seen: &mut BTreeSet<usize>, t: &mut Touch) {
        if !seen.insert(i) {
            return;
        }
        for r in &self.decls[i].refs {
            if r.phase == Phase::Cases || r.phase == Phase::Signature || (guards_only && r.phase != Phase::Guard) {
                continue;
            }
            match (r.kind, r.to) {
                (RefKind::Reads, Target::Decl(s)) if self.decls[s].kind == Kind::State => {
                    t.reads.insert(s);
                }
                (RefKind::Writes, Target::Decl(s)) => {
                    t.writes.insert(s);
                }
                (RefKind::Emits, Target::Out) => t.out = true,
                (RefKind::Calls, Target::Decl(c)) => self.touch_into(c, false, seen, t),
                _ => {}
            }
        }
    }

    /// A type as it is written, names resolved to their declarations.
    pub fn ty_text(&self, ty: &Ty) -> String {
        match ty {
            Ty::Unit => "()".to_string(),
            Ty::Bool => "Bool".to_string(),
            Ty::Int => "Int".to_string(),
            Ty::Text => "Text".to_string(),
            Ty::Named(id) => self.short_of(DeclRef::Type(*id)),
            Ty::Option(t) => format!("Option[{}]", self.ty_text(t)),
            Ty::List(t) => format!("List[{}]", self.ty_text(t)),
            Ty::Set(t) => format!("Set[{}]", self.ty_text(t)),
            Ty::Map(k, v) => format!("Map[{}, {}]", self.ty_text(k), self.ty_text(v)),
            Ty::Tuple(ts) => format!("({})", ts.iter().map(|t| self.ty_text(t)).collect::<Vec<_>>().join(", ")),
            Ty::Range { lo, hi, inclusive } => format!("{lo}{}{hi}", if *inclusive { "..=" } else { "..<" }),
            Ty::Where(base, claim) => format!("{} where {}", self.ty_text(base), self.src(claim.expr.span)),
            Ty::Trace(rules) => {
                let names: Vec<String> = rules.iter().map(|r| self.short_of(DeclRef::Rule(*r))).collect();
                format!("Trace[{}]", names.join(" | "))
            }
        }
    }

    /// The type under claims and aliases: `Title` gives `Text`, `Task` stays `Task`.
    pub fn underlying<'a>(&'a self, ty: &'a Ty) -> &'a Ty {
        let mut t = ty.strip_claims();
        for _ in 0..64 {
            match t {
                Ty::Named(id) => match &self.p.ty(*id).def {
                    TypeDef::Alias(inner) => t = inner.strip_claims(),
                    _ => return t,
                },
                _ => return t,
            }
        }
        t
    }

    /// The sum type a type is, if it is one.
    pub fn sum_of(&self, ty: &Ty) -> Option<TypeId> {
        match self.underlying(ty) {
            Ty::Named(id) if matches!(self.p.ty(*id).def, TypeDef::Sum(_)) => Some(*id),
            _ => None,
        }
    }

    /// The record type a type is, if it is one.
    pub fn record_of(&self, ty: &Ty) -> Option<TypeId> {
        match self.underlying(ty) {
            Ty::Named(id) if matches!(self.p.ty(*id).def, TypeDef::Record(_)) => Some(*id),
            _ => None,
        }
    }

    pub fn variant_name(&self, ty: TypeId, tag: u32) -> String {
        match &self.p.ty(ty).def {
            TypeDef::Sum(vs) => vs.get(tag as usize).map_or_else(|| format!("#{tag}"), |v| v.name.clone()),
            _ => format!("#{tag}"),
        }
    }

    /// The type of the value a place holds: the state's type, then one step per
    /// index or field. `None` where the IR does not say (a local, a dynamic step).
    pub fn place_ty(&self, place: &Place) -> Option<&Ty> {
        let PlaceRoot::State(s) = &place.root else {
            return None;
        };
        let mut ty = &self.p.state(*s).ty;
        for step in &place.path {
            ty = match (step, self.underlying(ty)) {
                (PlaceStep::Index(_), Ty::Map(_, v)) => v,
                (PlaceStep::Index(_), Ty::List(t)) => t,
                (PlaceStep::Field(f), Ty::Named(id)) => match &self.p.ty(*id).def {
                    TypeDef::Record(fields) => &fields.iter().find(|x| &x.name == f)?.ty,
                    _ => return None,
                },
                _ => return None,
            };
        }
        Some(ty)
    }
}

/// `todo.model` is `todo/model.slg`.
pub fn module_file(module: &str) -> String {
    format!("{}.slg", module.replace('.', "/"))
}

struct Collector<'a> {
    p: &'a Program,
    index: &'a HashMap<DeclRef, usize>,
    refs: Vec<Ref>,
    phase: Phase,
}

impl Collector<'_> {
    fn push(&mut self, kind: RefKind, to: Target) {
        let r = Ref { kind, to, phase: self.phase };
        if !self.refs.contains(&r) {
            self.refs.push(r);
        }
    }

    /// A reference to a declaration; one to the built-in core module is dropped.
    fn to(&mut self, kind: RefKind, r: DeclRef) {
        if let Some(&i) = self.index.get(&r) {
            self.push(kind, Target::Decl(i));
        }
    }

    fn decl(&mut self, r: DeclRef) {
        let p = self.p;
        match r {
            DeclRef::Type(id) => match &p.ty(id).def {
                TypeDef::Alias(ty) => self.ty(ty),
                TypeDef::Record(fields) => self.fields(fields),
                TypeDef::Sum(variants) => variants.iter().for_each(|v| self.fields(&v.fields)),
                TypeDef::Rules(rules) => rules.iter().for_each(|r| self.to(RefKind::Traces, DeclRef::Rule(*r))),
            },
            DeclRef::Const(id) => {
                let c = p.constant(id);
                c.ty.iter().for_each(|t| self.ty(t));
                self.phase = Phase::Body;
                self.expr(&c.value);
            }
            DeclRef::State(id) => {
                let s = p.state(id);
                self.ty(&s.ty);
                self.phase = Phase::Body;
                s.init.iter().for_each(|e| self.expr(e));
            }
            DeclRef::Fn(id) => {
                let f = p.func(id);
                self.params(&f.params);
                f.result.iter().for_each(|t| self.ty(t));
                self.phase = Phase::Body;
                self.expr(&f.body);
                self.cases(&f.cases);
            }
            DeclRef::Rule(id) => {
                let rule = p.rule(id);
                self.params(&rule.params);
                rule.result.iter().for_each(|t| self.ty(t));
                self.phase = Phase::Guard;
                rule.guards.iter().for_each(|g| self.expr(&g.cond));
                self.phase = Phase::Body;
                self.expr(&rule.body);
            }
            DeclRef::Test(id) => {
                let t = p.test(id);
                self.params(&t.params);
                for g in &t.given {
                    match g {
                        Given::State { state, value, .. } => {
                            self.to(RefKind::Sets, DeclRef::State(*state));
                            self.expr(value);
                        }
                        Given::Test { test, .. } => self.to(RefKind::Given, DeclRef::Test(*test)),
                    }
                }
                self.phase = Phase::Body;
                self.block(&t.body);
                self.cases(&t.cases);
            }
            DeclRef::Law(id) => {
                let l = p.law(id);
                self.params(&l.params);
                self.phase = Phase::Body;
                self.expr(&l.body);
            }
            DeclRef::App(id) => p.app(id).rules.iter().for_each(|r| self.to(RefKind::Exposes, DeclRef::Rule(*r))),
        }
    }

    fn cases(&mut self, cases: &[spec_lang_ir::Case]) {
        self.phase = Phase::Cases;
        for c in cases {
            c.args.iter().flatten().for_each(|e| self.expr(e));
            self.expect(&c.expect);
        }
    }

    fn params(&mut self, params: &[spec_lang_ir::Param]) {
        for x in params {
            self.ty(&x.ty);
            x.default.iter().for_each(|e| self.expr(e));
        }
    }

    fn fields(&mut self, fields: &[spec_lang_ir::Field]) {
        for x in fields {
            self.ty(&x.ty);
            x.default.iter().for_each(|e| self.expr(e));
        }
    }

    fn ty(&mut self, ty: &Ty) {
        match ty {
            Ty::Unit | Ty::Bool | Ty::Int | Ty::Text | Ty::Range { .. } => {}
            Ty::Named(id) => self.to(RefKind::Uses, DeclRef::Type(*id)),
            Ty::Option(t) | Ty::List(t) | Ty::Set(t) => self.ty(t),
            Ty::Map(k, v) => {
                self.ty(k);
                self.ty(v);
            }
            Ty::Tuple(ts) => ts.iter().for_each(|t| self.ty(t)),
            Ty::Where(base, claim) => {
                self.ty(base);
                self.expr(&claim.expr);
            }
            Ty::Trace(rules) => rules.iter().for_each(|r| self.to(RefKind::Traces, DeclRef::Rule(*r))),
        }
    }

    fn expect(&mut self, e: &Expect) {
        match e {
            Expect::Value(v) => self.expr(v),
            Expect::Fail(f) => f.args.iter().for_each(|a| self.expr(a)),
        }
    }

    fn block(&mut self, b: &Block) {
        b.stmts.iter().for_each(|s| self.stmt(s));
    }

    fn stmt(&mut self, s: &Stmt) {
        match &s.kind {
            StmtKind::Let { ty, value, .. } => {
                ty.iter().for_each(|t| self.ty(t));
                self.expr(value);
            }
            StmtKind::Assign { place, value } => {
                self.expr(value);
                self.place(place);
            }
            StmtKind::Mutate { place, args, .. } => {
                args.iter().for_each(|a| self.expr(a));
                self.place(place);
            }
            StmtKind::Expr(e) | StmtKind::Assert(e) => self.expr(e),
            StmtKind::Return(e) => e.iter().for_each(|e| self.expr(e)),
            StmtKind::For { iter, body, .. } => {
                self.expr(iter);
                self.block(body);
            }
            StmtKind::While { cond, body } => {
                self.expr(cond);
                self.block(body);
            }
            StmtKind::If { cond, then, els } => {
                self.expr(cond);
                self.block(then);
                els.iter().for_each(|b| self.block(b));
            }
            StmtKind::Expect { subject, expect } => {
                self.expr(subject);
                self.expect(expect);
            }
        }
    }

    fn place(&mut self, place: &Place) {
        for step in &place.path {
            if let PlaceStep::Index(key) = step {
                self.expr(key);
            }
        }
        if let PlaceRoot::State(s) = &place.root {
            self.to(RefKind::Writes, DeclRef::State(*s));
        }
    }

    fn pat_args(&mut self, args: &[PatArg]) {
        args.iter().for_each(|a| self.pattern(&a.pattern));
    }

    fn pattern(&mut self, pat: &Pattern) {
        match pat {
            Pattern::Wild | Pattern::Bind(_) => {}
            Pattern::Value(e) => self.expr(e),
            Pattern::Variant { ty, args, .. } => {
                self.to(RefKind::Matches, DeclRef::Type(*ty));
                self.pat_args(args);
            }
            Pattern::Tuple(ps) => ps.iter().for_each(|x| self.pattern(x)),
            Pattern::Call { rule, args } => {
                self.to(RefKind::Matches, DeclRef::Rule(*rule));
                self.pat_args(args);
            }
        }
    }

    fn arm(&mut self, arm: &Arm) {
        self.pattern(&arm.pattern);
        self.expr(&arm.value);
    }

    fn expr(&mut self, e: &Expr) {
        match &e.kind {
            ExprKind::Unit
            | ExprKind::Bool(_)
            | ExprKind::Int(_)
            | ExprKind::Text(_)
            | ExprKind::Local(_)
            | ExprKind::Hole(_) => {}
            ExprKind::OutText => self.push(RefKind::Reads, Target::Out),
            ExprKind::Interp(parts) => {
                for part in parts {
                    if let InterpPart::Expr(x) = part {
                        self.expr(x);
                    }
                }
            }
            ExprKind::Const(id) => self.to(RefKind::Reads, DeclRef::Const(*id)),
            ExprKind::State(id) => self.to(RefKind::Reads, DeclRef::State(*id)),
            ExprKind::Variant { ty, .. } => self.to(RefKind::Constructs, DeclRef::Type(*ty)),
            ExprKind::Call { callee, args } => {
                match callee {
                    Callee::Fn(id) => self.to(RefKind::Calls, DeclRef::Fn(*id)),
                    Callee::Rule(id) => self.to(RefKind::Calls, DeclRef::Rule(*id)),
                    Callee::Record(id) | Callee::Variant(id, _) => self.to(RefKind::Constructs, DeclRef::Type(*id)),
                    Callee::Effect(Effect::OutLine) => self.push(RefKind::Emits, Target::Out),
                }
                args.iter().flatten().for_each(|a| self.expr(a));
            }
            ExprKind::CallValue { callee, args } => {
                self.expr(callee);
                args.iter().for_each(|a| self.expr(a));
            }
            ExprKind::Method { recv, args, .. } => {
                self.expr(recv);
                args.iter().for_each(|a| self.expr(&a.value));
            }
            ExprKind::Prop { recv, .. } => self.expr(recv),
            ExprKind::Index { recv, key } => {
                self.expr(recv);
                self.expr(key);
            }
            ExprKind::With { recv, fields } => {
                self.expr(recv);
                fields.iter().for_each(|(_, v)| self.expr(v));
            }
            ExprKind::Is { value, pattern } => {
                self.expr(value);
                self.pattern(pattern);
            }
            ExprKind::Unary(_, x) => self.expr(x),
            ExprKind::Binary(_, a, b) => {
                self.expr(a);
                self.expr(b);
            }
            ExprKind::Compare(first, rest) => {
                self.expr(first);
                rest.iter().for_each(|(_, x)| self.expr(x));
            }
            ExprKind::Range { lo, hi, .. } => {
                self.expr(lo);
                self.expr(hi);
            }
            ExprKind::Tuple(xs) | ExprKind::List(xs) | ExprKind::SetOf(xs) => xs.iter().for_each(|x| self.expr(x)),
            ExprKind::MapOf(pairs) => {
                for (k, v) in pairs {
                    self.expr(k);
                    self.expr(v);
                }
            }
            ExprKind::Lambda(id) => match &self.p.lambda(*id).body {
                LambdaBody::Expr(x) => self.expr(x),
                LambdaBody::Block(b) => self.block(b),
            },
            ExprKind::If { cond, then, els } => {
                self.expr(cond);
                self.expr(then);
                self.expr(els);
            }
            ExprKind::Else { value, fallback, .. } => {
                self.expr(value);
                self.expr(fallback);
            }
            ExprKind::Fail(f) => f.args.iter().for_each(|a| self.expr(a)),
            ExprKind::Match { scrutinee, arms } => {
                self.expr(scrutinee);
                arms.iter().for_each(|a| self.arm(a));
            }
            ExprKind::Block(b) => self.block(b),
        }
    }
}

/// Every statement of a block, nested blocks included, in source order.
pub fn each_stmt<'a>(b: &'a Block, out: &mut Vec<&'a Stmt>) {
    for s in &b.stmts {
        out.push(s);
        match &s.kind {
            StmtKind::For { body, .. } | StmtKind::While { body, .. } => each_stmt(body, out),
            StmtKind::If { then, els, .. } => {
                each_stmt(then, out);
                els.iter().for_each(|e| each_stmt(e, out));
            }
            StmtKind::Let { .. }
            | StmtKind::Assign { .. }
            | StmtKind::Mutate { .. }
            | StmtKind::Expr(_)
            | StmtKind::Return(_)
            | StmtKind::Assert(_)
            | StmtKind::Expect { .. } => {}
        }
    }
}

/// The expressions a statement holds directly (not those of nested blocks).
pub fn stmt_exprs<'a>(s: &'a Stmt, out: &mut Vec<&'a Expr>) {
    match &s.kind {
        StmtKind::Let { value, .. } => out.push(value),
        StmtKind::Assign { place, value } => {
            place_exprs(place, out);
            out.push(value);
        }
        StmtKind::Mutate { place, args, .. } => {
            place_exprs(place, out);
            out.extend(args);
        }
        StmtKind::Expr(e) | StmtKind::Assert(e) => out.push(e),
        StmtKind::Return(e) => out.extend(e),
        StmtKind::For { iter: e, .. } | StmtKind::While { cond: e, .. } | StmtKind::If { cond: e, .. } => out.push(e),
        StmtKind::Expect { subject, expect } => {
            out.push(subject);
            match expect {
                Expect::Value(v) => out.push(v),
                Expect::Fail(f) => out.extend(&f.args),
            }
        }
    }
}

fn place_exprs<'a>(place: &'a Place, out: &mut Vec<&'a Expr>) {
    for step in &place.path {
        if let PlaceStep::Index(key) = step {
            out.push(key);
        }
    }
}

fn block_exprs<'a>(b: &'a Block, out: &mut Vec<&'a Expr>) {
    let mut stmts = Vec::new();
    each_stmt(b, &mut stmts);
    stmts.into_iter().for_each(|s| stmt_exprs(s, out));
}

fn pattern_exprs<'a>(pat: &'a Pattern, out: &mut Vec<&'a Expr>) {
    match pat {
        Pattern::Wild | Pattern::Bind(_) => {}
        Pattern::Value(e) => out.push(e),
        Pattern::Variant { args, .. } | Pattern::Call { args, .. } => {
            args.iter().for_each(|a| pattern_exprs(&a.pattern, out));
        }
        Pattern::Tuple(ps) => ps.iter().for_each(|x| pattern_exprs(x, out)),
    }
}

/// Visit an expression and everything written inside it, outermost first, in
/// source order: lambda bodies and block statements included.
pub fn walk<'a>(p: &'a Program, e: &'a Expr, visit: &mut dyn FnMut(&'a Expr)) {
    visit(e);
    let mut kids: Vec<&'a Expr> = Vec::new();
    match &e.kind {
        ExprKind::Unit
        | ExprKind::Bool(_)
        | ExprKind::Int(_)
        | ExprKind::Text(_)
        | ExprKind::Local(_)
        | ExprKind::Hole(_)
        | ExprKind::OutText
        | ExprKind::Const(_)
        | ExprKind::State(_)
        | ExprKind::Variant { .. } => {}
        ExprKind::Interp(parts) => {
            for part in parts {
                if let InterpPart::Expr(x) = part {
                    kids.push(x);
                }
            }
        }
        ExprKind::Call { args, .. } => kids.extend(args.iter().flatten()),
        ExprKind::CallValue { callee, args } => {
            kids.push(callee);
            kids.extend(args);
        }
        ExprKind::Method { recv, args, .. } => {
            kids.push(recv);
            kids.extend(args.iter().map(|a| &a.value));
        }
        ExprKind::Prop { recv, .. } => kids.push(recv),
        ExprKind::Index { recv, key } => kids.extend([recv.as_ref(), key.as_ref()]),
        ExprKind::With { recv, fields } => {
            kids.push(recv);
            kids.extend(fields.iter().map(|(_, v)| v));
        }
        ExprKind::Is { value, pattern } => {
            kids.push(value);
            pattern_exprs(pattern, &mut kids);
        }
        ExprKind::Unary(_, x) => kids.push(x),
        ExprKind::Binary(_, a, b) => kids.extend([a.as_ref(), b.as_ref()]),
        ExprKind::Compare(first, rest) => {
            kids.push(first);
            kids.extend(rest.iter().map(|(_, x)| x));
        }
        ExprKind::Range { lo, hi, .. } => kids.extend([lo.as_ref(), hi.as_ref()]),
        ExprKind::Tuple(xs) | ExprKind::List(xs) | ExprKind::SetOf(xs) => kids.extend(xs),
        ExprKind::MapOf(pairs) => pairs.iter().for_each(|(k, v)| kids.extend([k, v])),
        ExprKind::Lambda(id) => match &p.lambda(*id).body {
            LambdaBody::Expr(x) => kids.push(x),
            LambdaBody::Block(b) => block_exprs(b, &mut kids),
        },
        ExprKind::If { cond, then, els } => kids.extend([cond.as_ref(), then.as_ref(), els.as_ref()]),
        ExprKind::Else { value, fallback, .. } => kids.extend([value.as_ref(), fallback.as_ref()]),
        ExprKind::Fail(f) => kids.extend(&f.args),
        ExprKind::Match { scrutinee, arms } => {
            kids.push(scrutinee);
            for a in arms {
                pattern_exprs(&a.pattern, &mut kids);
                kids.push(&a.value);
            }
        }
        ExprKind::Block(b) => block_exprs(b, &mut kids),
    }
    for k in kids {
        walk(p, k, visit);
    }
}
