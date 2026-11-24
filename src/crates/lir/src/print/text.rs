//! The LIR as readable text: one declaration per paragraph, expressions in
//! a compact prefix form (`list.filter(xs, (t: Task) => ...)`), every
//! binding with its type. Stable: same program, same bytes.

use crate::manifest::{Named, Note, NoteValue};
use crate::node::*;
use crate::program::*;
use crate::ty::{DTy, LTy};
use spec_lang_ir::{Program, TypeId};

/// An expression type as text, with type names from the IR (for messages).
pub(crate) fn ty(prog: &Program, t: &LTy) -> String {
    ty_with(t, &|id| prog.qualified(&prog.ty(id).info).trim_start_matches("core.").to_string())
}

fn ty_with(t: &LTy, name: &dyn Fn(TypeId) -> String) -> String {
    let list = |ts: &[LTy]| ts.iter().map(|x| ty_with(x, name)).collect::<Vec<_>>().join(", ");
    let rules = |rs: &[spec_lang_ir::RuleId]| rs.iter().map(|r| format!("#{}", r.0)).collect::<Vec<_>>().join(" | ");
    match t {
        LTy::Unit => "()".into(),
        LTy::Bool => "Bool".into(),
        LTy::Int => "Int".into(),
        LTy::Text => "Text".into(),
        LTy::Named(id) => name(*id),
        LTy::Option(x) => format!("Option[{}]", ty_with(x, name)),
        LTy::List(x) => format!("List[{}]", ty_with(x, name)),
        LTy::Set(x) => format!("Set[{}]", ty_with(x, name)),
        LTy::Map(k, v) => format!("Map[{}, {}]", ty_with(k, name), ty_with(v, name)),
        LTy::Tuple(ts) => format!("({})", list(ts)),
        LTy::Range => "Range".into(),
        LTy::Func(ps, r) => format!("({}) -> {}", list(ps), ty_with(r, name)),
        LTy::Event(rs) => format!("Event[{}]", rules(rs)),
        LTy::Reply(rs) => format!("Reply[{}]", rules(rs)),
        LTy::Failure => "Failure".into(),
        LTy::Step(rs) => format!("Step[{}]", rules(rs)),
        LTy::World => "World".into(),
        LTy::Trace(rs) => format!("Trace[{}]", rules(rs)),
        LTy::Never => "Never".into(),
        LTy::Var(v) => format!("?{v}"),
    }
}

/// Renders a whole program.
#[derive(Debug)]
struct Printer<'l> {
    lir: &'l Lir,
    out: String,
}

/// The whole program as text.
pub fn program(lir: &Lir) -> String {
    let mut p = Printer { lir, out: String::new() };
    p.all();
    p.out
}

impl Printer<'_> {
    fn name(&self, id: TypeId) -> String {
        let t = self.lir.ty(id);
        if t.builtin { t.info.name.clone() } else { t.info.qualified.clone() }
    }

    fn lty(&self, t: &LTy) -> String {
        ty_with(t, &|id| self.name(id))
    }

    fn dty(&self, t: &DTy) -> String {
        match t {
            DTy::Unit => "()".into(),
            DTy::Bool => "Bool".into(),
            DTy::Int => "Int".into(),
            DTy::Text => "Text".into(),
            DTy::Named(id) => self.name(*id),
            DTy::Option(x) => format!("Option[{}]", self.dty(x)),
            DTy::List(x) => format!("List[{}]", self.dty(x)),
            DTy::Set(x) => format!("Set[{}]", self.dty(x)),
            DTy::Map(k, v) => format!("Map[{}, {}]", self.dty(k), self.dty(v)),
            DTy::Tuple(ts) => format!("({})", ts.iter().map(|x| self.dty(x)).collect::<Vec<_>>().join(", ")),
            DTy::Range { lo, hi, inclusive } => format!("{lo}{}{hi}", if *inclusive { "..=" } else { "..<" }),
            DTy::Where(b, c) => format!("{} where claim#{}", self.dty(b), c.0),
            DTy::Trace(rs) => {
                format!(
                    "Trace[{}]",
                    rs.iter().map(|r| self.lir.rule(*r).info.qualified.clone()).collect::<Vec<_>>().join(" | ")
                )
            }
        }
    }

    fn line(&mut self, s: &str) {
        self.out.push_str(s);
        self.out.push('\n');
    }

    fn all(&mut self) {
        let lir = self.lir;
        for m in lir.modules.iter().skip(1) {
            let v =
                if m.vocabularies.is_empty() { String::new() } else { format!(" uses {}", m.vocabularies.join(", ")) };
            self.line(&format!("module {} ({}){v}", m.name, m.path));
        }
        self.line("");
        for t in lir.types.iter().filter(|t| !t.builtin) {
            let s = self.type_decl(t);
            self.line(&s);
        }
        for (i, c) in lir.claims.iter().enumerate() {
            let s = format!("claim#{i}(self: {}) = {}    // {}", self.lty(&c.self_ty), self.expr(&c.body, 1), c.text);
            self.line(&s);
        }
        self.line("");
        for c in &lir.consts {
            let s = format!("const {}: {} = {}", c.info.qualified, self.lty(&c.ty), self.expr(&c.value, 1));
            self.line(&s);
        }
        for s in &lir.states {
            let init = match &s.init {
                Some(e) => self.expr(e, 1),
                None => "empty".into(),
            };
            let l = format!("state {}: {} check {} = {init}", s.info.qualified, self.lty(&s.ty), self.dty(&s.check));
            self.line(&l);
        }
        self.line("");
        for f in &lir.fns {
            let head = format!(
                "fn {}({}) -> {}",
                f.info.qualified,
                self.params(&f.params),
                self.result(&f.result, &f.result_check)
            );
            self.line(&head);
            let b = self.body(&f.body);
            self.line(&format!("  = {b}"));
        }
        for r in &lir.rules {
            let trig: Vec<String> = r.triggers.iter().map(note).collect();
            let head = format!(
                "rule {}({}) -> {}{}",
                r.info.qualified,
                self.params(&r.params),
                self.result(&r.result, &r.result_check),
                if trig.is_empty() { String::new() } else { format!(" on {}", trig.join(", ")) }
            );
            self.line(&head);
            for g in &r.guards {
                let s = format!("  guard {} else {}", self.expr(&g.cond, 2), g.error);
                self.line(&s);
            }
            let b = self.body(&r.body);
            self.line(&format!("  = {b}"));
        }
        self.line("");
        for t in &lir.tests {
            self.line(&format!("test {}({})", t.info.qualified, self.params(&t.params)));
            for g in &t.given {
                let s = match g {
                    Given::State { state, value } => {
                        format!("  given {} -> {}", lir.state(*state).info.qualified, self.expr(value, 2))
                    }
                    Given::Test(id) => format!("  given test {}", lir.test(*id).info.qualified),
                };
                self.line(&s);
            }
            let b = self.block(&t.body, 1);
            self.line(&format!("  = {b}"));
        }
        for l in &lir.laws {
            let traced: Vec<String> = l.traced.iter().map(|r| lir.rule(*r).info.qualified.clone()).collect();
            let start: Vec<String> = l.start_rules.iter().map(|r| lir.rule(*r).info.qualified.clone()).collect();
            self.line(&format!("law {}({})", l.info.qualified, self.params(&l.params)));
            self.line(&format!("  traces {}; start worlds from {}", traced.join(" | "), start.join(", ")));
            let b = self.expr(&l.body, 2);
            self.line(&format!("  = {b}"));
        }
        self.line("");
        for c in &lir.checks {
            let s = match &c.what {
                CheckWhat::Case { target, args, expect } => {
                    let callee = match target {
                        CaseTarget::Fn(f) => lir.func(*f).info.qualified.clone(),
                        CaseTarget::Test(t) => lir.test(*t).info.qualified.clone(),
                    };
                    format!(
                        "check {} [case]: {callee}({}) -> {}",
                        c.name,
                        self.opt_args(args, 1),
                        self.expected(expect, 1)
                    )
                }
                other => format!(
                    "check {} [{}]",
                    c.name,
                    match other {
                        CheckWhat::Property(_) => "property",
                        CheckWhat::Law(_) => "law",
                        _ => "test",
                    }
                ),
            };
            self.line(&s);
        }
        self.line("");
        for a in &lir.apps {
            let notes: Vec<String> = a.notes.iter().map(note).collect();
            self.line(&format!(
                "app {} target {} notes [{}]",
                a.qualified,
                a.target.clone().unwrap_or_else(|| "-".into()),
                notes.join(", ")
            ));
            for p in &a.params {
                let ds: Vec<String> = p
                    .decls
                    .iter()
                    .map(|d| match d {
                        Named::Rule(r) => lir.rule(*r).info.qualified.clone(),
                        Named::Fn(f) => lir.func(*f).info.qualified.clone(),
                    })
                    .collect();
                self.line(&format!("  {}: {}", p.name, ds.join(", ")));
            }
            let rs: Vec<String> = a.rules.iter().map(|r| lir.rule(*r).info.qualified.clone()).collect();
            self.line(&format!("  reaches {}", rs.join(", ")));
        }
    }

    fn type_decl(&self, t: &TypeDecl) -> String {
        let fields = |fs: &[Field]| -> String {
            fs.iter()
                .map(|f| {
                    let d = f.default.as_ref().map(|e| format!(" = {}", self.expr(e, 1))).unwrap_or_default();
                    format!("{}: {}{d}", f.name, self.dty(&f.check))
                })
                .collect::<Vec<_>>()
                .join(", ")
        };
        match &t.def {
            TypeDef::Alias { of } => format!("type {} = {}", t.info.qualified, self.dty(of)),
            TypeDef::Record { fields: fs } => format!("type {}({})", t.info.qualified, fields(fs)),
            TypeDef::Sum { variants } => {
                let vs: Vec<String> = variants
                    .iter()
                    .map(|v| {
                        if v.fields.is_empty() { v.name.clone() } else { format!("{}({})", v.name, fields(&v.fields)) }
                    })
                    .collect();
                format!("type {} = {}", t.info.qualified, vs.join(" | "))
            }
            TypeDef::RuleSet { rules } => format!(
                "type {} = rules {}",
                t.info.qualified,
                rules.iter().map(|r| self.lir.rule(*r).info.qualified.clone()).collect::<Vec<_>>().join(" | ")
            ),
        }
    }

    fn params(&self, ps: &[Param]) -> String {
        ps.iter()
            .map(|p| {
                let d = p.default.as_ref().map(|e| format!(" = {}", self.expr(e, 1))).unwrap_or_default();
                format!("{}: {} check {}{d}", p.name, self.lty(&p.ty), self.dty(&p.check))
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn result(&self, t: &LTy, check: &Option<DTy>) -> String {
        match check {
            Some(c) => format!("{} check {}", self.lty(t), self.dty(c)),
            None => self.lty(t),
        }
    }

    fn body(&self, b: &Body) -> String {
        match b {
            Body::Expr(e) => self.expr(e, 1),
            Body::Extern(x) => {
                format!("extern {} \"{}#{}\" effects [{}]", x.lang, x.library, x.symbol, x.effects.join(", "))
            }
        }
    }

    fn args(&self, xs: &[Expr], d: usize) -> String {
        xs.iter().map(|x| self.expr(x, d)).collect::<Vec<_>>().join(", ")
    }

    fn opt_args(&self, xs: &[Option<Expr>], d: usize) -> String {
        xs.iter().map(|x| x.as_ref().map_or("default".to_string(), |e| self.expr(e, d))).collect::<Vec<_>>().join(", ")
    }

    fn expected(&self, e: &Expected, d: usize) -> String {
        match e {
            Expected::Value(x) => self.expr(x, d),
            Expected::Fail { name, args } if args.is_empty() => format!("fail {name}"),
            Expected::Fail { name, args } => format!("fail {name}({})", self.args(args, d)),
        }
    }

    fn path(&self, p: &[PathStep]) -> String {
        let mut s = "$".to_string();
        for step in p {
            s.push_str(&match step {
                PathStep::VariantField { ty, tag, index } => format!(".{}#{tag}.{index}", self.name(*ty)),
                PathStep::TupleItem(i) => format!(".{i}"),
                PathStep::EventArg { rule, index } => format!(".{}.{index}", self.lir.rule(*rule).info.qualified),
            });
        }
        s
    }

    fn pat(&self, p: &Pat, d: usize) -> String {
        if p.steps.is_empty() {
            return "_".into();
        }
        p.steps
            .iter()
            .map(|s| match s {
                PatStep::Tag { path, ty, tag } => {
                    let v = match &self.lir.ty(*ty).def {
                        TypeDef::Sum { variants } => variants.get(*tag as usize).map_or("?".into(), |v| v.name.clone()),
                        _ => "?".into(),
                    };
                    format!("{} is {v}", self.path(path))
                }
                PatStep::Event { path, rule } => {
                    format!("{} is {}", self.path(path), self.lir.rule(*rule).info.qualified)
                }
                PatStep::Equals { path, value } => format!("{} == {}", self.path(path), self.expr(value, d)),
                PatStep::Never => "never".into(),
                PatStep::Bind { name, path, ty } => format!("let {name}: {} = {}", self.lty(ty), self.path(path)),
            })
            .collect::<Vec<_>>()
            .join("; ")
    }

    fn expr(&self, e: &Expr, d: usize) -> String {
        let lir = self.lir;
        match &e.kind {
            ExprKind::Unit => "()".into(),
            ExprKind::Bool(b) => b.to_string(),
            ExprKind::Int(n) => n.to_string(),
            ExprKind::Text(t) => format!("{t:?}"),
            ExprKind::Interp(parts) => {
                let ps: Vec<String> = parts
                    .iter()
                    .map(|p| match p {
                        Part::Lit(s) => format!("{s:?}"),
                        Part::Show(x) => format!("show({})", self.expr(x, d)),
                    })
                    .collect();
                format!("concat({})", ps.join(", "))
            }
            ExprKind::Local(n) => n.clone(),
            ExprKind::Const(c) => lir.constant(*c).info.qualified.clone(),
            ExprKind::State(s) => format!("state {}", lir.state(*s).info.qualified),
            ExprKind::Variant { ty, tag } => self.variant(*ty, *tag),
            ExprKind::Some(x) => format!("Some({})", self.expr(x, d)),
            ExprKind::None => format!("none: {}", self.lty(&e.ty)),
            ExprKind::Construct { ty, tag, args } => {
                let n = match tag {
                    Some(t) => self.variant(*ty, *t),
                    None => self.name(*ty),
                };
                format!("new {n}({})", self.opt_args(args, d))
            }
            ExprKind::CallFn { func, args } => {
                format!("call {}({})", lir.func(*func).info.qualified, self.opt_args(args, d))
            }
            ExprKind::CallRule { rule, args } => {
                format!("rule {}({})", lir.rule(*rule).info.qualified, self.opt_args(args, d))
            }
            ExprKind::Effect { name, args, .. } => format!("effect {name}({})", self.args(args, d)),
            ExprKind::EffectRead { name, .. } => format!("effect {name}"),
            ExprKind::Prim { op, args } => format!("{}({})", op.name(), self.args(args, d)),
            ExprKind::Lambda(l) => {
                let ps: Vec<String> = l.params.iter().map(|(n, t)| format!("{n}: {}", self.lty(t))).collect();
                let cs: Vec<String> = l.captures.iter().map(|(n, _)| n.clone()).collect();
                let body = match &l.body {
                    LambdaBody::Expr(x) => self.expr(x, d),
                    LambdaBody::Block(b) => self.block(b, d),
                };
                let cap = if cs.is_empty() { String::new() } else { format!(" [{}]", cs.join(", ")) };
                format!("({}){cap} -> {} => {body}", ps.join(", "), self.lty(&l.result))
            }
            ExprKind::CallLambda { callee, args } => format!("apply({}, {})", self.expr(callee, d), self.args(args, d)),
            ExprKind::Field { recv, name, index, .. } => format!("{}.{name}#{index}", self.expr(recv, d)),
            ExprKind::VariantField { recv, name, .. } => format!("{}.{name}", self.expr(recv, d)),
            ExprKind::WorldState { recv, state } => {
                format!("{}[{}]", self.expr(recv, d), lir.state(*state).info.qualified)
            }
            ExprKind::EventArg { recv, name, .. } => format!("{}.arg({name})", self.expr(recv, d)),
            ExprKind::With { recv, changes, .. } => {
                let cs: Vec<String> = changes.iter().map(|(i, x)| format!("#{i}: {}", self.expr(x, d))).collect();
                format!("{}.with({})", self.expr(recv, d), cs.join(", "))
            }
            ExprKind::Is { value, pat } => format!("({} matches {{{}}})", self.expr(value, d), self.pat(pat, d)),
            ExprKind::If { cond, then, els } => {
                format!("(if {} then {} else {})", self.expr(cond, d), self.expr(then, d), self.expr(els, d))
            }
            ExprKind::Logic { op, left, right } => {
                format!("({} {} {})", self.expr(left, d), op.name(), self.expr(right, d))
            }
            ExprKind::Compare { first, links } => {
                let mut s = self.expr(first, d);
                for (op, x) in links {
                    s.push_str(&format!(" {} {}", op.name(), self.expr(x, d)));
                }
                format!("({s})")
            }
            ExprKind::Else { value, fallback, unwrap } => format!(
                "({} {} {})",
                self.expr(value, d),
                if *unwrap { "else_unwrap" } else { "else" },
                self.expr(fallback, d)
            ),
            ExprKind::Fail { name, args } if args.is_empty() => format!("fail {name}"),
            ExprKind::Fail { name, args } => format!("fail {name}({})", self.args(args, d)),
            ExprKind::Match { scrutinee, arms } => {
                let pad = "  ".repeat(d + 1);
                let mut s = format!("match {} {{\n", self.expr(scrutinee, d));
                for a in arms {
                    s.push_str(&format!("{pad}{{{}}} => {}\n", self.pat(&a.pat, d + 1), self.expr(&a.value, d + 1)));
                }
                s.push_str(&"  ".repeat(d));
                s.push('}');
                s
            }
            ExprKind::Block(b) => self.block(b, d),
            ExprKind::Hole(h) => format!("?{h}"),
            ExprKind::Tuple(xs) => format!("({})", self.args(xs, d)),
            ExprKind::List(xs) => format!("[{}]: {}", self.args(xs, d), self.lty(&e.ty)),
            ExprKind::SetOf(xs) => format!("Set({}): {}", self.args(xs, d), self.lty(&e.ty)),
            ExprKind::MapOf(ps) => {
                let s: Vec<String> =
                    ps.iter().map(|(k, v)| format!("{} -> {}", self.expr(k, d), self.expr(v, d))).collect();
                format!("Map({}): {}", s.join(", "), self.lty(&e.ty))
            }
            ExprKind::Range { lo, hi, inclusive } => {
                format!("{}{}{}", self.expr(lo, d), if *inclusive { "..=" } else { "..<" }, self.expr(hi, d))
            }
        }
    }

    fn variant(&self, ty: TypeId, tag: u32) -> String {
        match &self.lir.ty(ty).def {
            TypeDef::Sum { variants } => {
                format!("{}.{}", self.name(ty), variants.get(tag as usize).map_or("?", |v| v.name.as_str()))
            }
            _ => "?".into(),
        }
    }

    fn place(&self, p: &Place, d: usize) -> String {
        let mut s = match &p.root {
            PlaceRoot::State(id) => format!("state {}", self.lir.state(*id).info.qualified),
            PlaceRoot::Local(n) => n.clone(),
        };
        for step in &p.path {
            s.push_str(&match step {
                PlaceStep::Field { index, .. } => format!(".#{index}"),
                PlaceStep::MapKey(k) => format!("[key {}]", self.expr(k, d)),
                PlaceStep::ListIndex(k) => format!("[at {}]", self.expr(k, d)),
            });
        }
        if let Some(c) = &p.check {
            s.push_str(&format!(" check {}", self.dty(c)));
        }
        s
    }

    fn block(&self, b: &Block, d: usize) -> String {
        let pad = "  ".repeat(d + 1);
        let mut s = format!("{{ // gives {}\n", self.lty(&b.ty));
        for st in &b.stmts {
            s.push_str(&pad);
            s.push_str(&self.stmt(st, d + 1));
            s.push('\n');
        }
        s.push_str(&"  ".repeat(d));
        s.push('}');
        s
    }

    fn stmt(&self, s: &Stmt, d: usize) -> String {
        match &s.kind {
            StmtKind::Let { name, mutable, ty, check, value } => {
                let c = check.as_ref().map(|c| format!(" check {}", self.dty(c))).unwrap_or_default();
                format!(
                    "{} {name}: {}{c} = {};",
                    if *mutable { "var" } else { "let" },
                    self.lty(ty),
                    self.expr(value, d)
                )
            }
            StmtKind::Assign { place, value } => format!("{} := {};", self.place(place, d), self.expr(value, d)),
            StmtKind::Mutate { place, op, args } => {
                format!(
                    "{} := {}(<place>{});",
                    self.place(place, d),
                    op.name(),
                    args.iter().map(|a| format!(", {}", self.expr(a, d))).collect::<String>()
                )
            }
            StmtKind::Expr(e) => format!("{};", self.expr(e, d)),
            StmtKind::Return(None) => "return;".into(),
            StmtKind::Return(Some(e)) => format!("return {};", self.expr(e, d)),
            StmtKind::Assert(e) => format!("assert {};", self.expr(e, d)),
            StmtKind::For { var, ty, iter, body } => {
                format!("for {var}: {} in {} {}", self.lty(ty), self.expr(iter, d), self.block(body, d))
            }
            StmtKind::While { cond, body } => format!("while {} {}", self.expr(cond, d), self.block(body, d)),
            StmtKind::If { cond, then, els } => {
                let e = els.as_ref().map(|b| format!(" else {}", self.block(b, d))).unwrap_or_default();
                format!("if {} {}{e}", self.expr(cond, d), self.block(then, d))
            }
            StmtKind::Expect { subject, expect } => {
                format!("expect {} -> {};", self.expr(subject, d), self.expected(expect, d))
            }
        }
    }
}

fn note_value(v: &NoteValue) -> String {
    match v {
        NoteValue::Int(i) => i.to_string(),
        NoteValue::Text(t) => format!("\"{t}\""),
        NoteValue::Quantity { value, unit } => format!("{value}{unit}"),
        NoteValue::Variant { name, args } if args.is_empty() => name.clone(),
        NoteValue::Variant { name, args } => {
            format!("{name}({})", args.iter().map(|a| note_value(&a.value)).collect::<Vec<_>>().join(", "))
        }
    }
}

/// A note as written: `cli("add")`, `tick(60Hz)`.
pub fn note(n: &Note) -> String {
    let args: Vec<String> = n.args.iter().map(|a| note_value(&a.value)).collect();
    format!("{}({})", n.name, args.join(", "))
}
