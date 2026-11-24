//! Expressions. Evaluation is left to right as the language says: where C++
//! leaves the order of siblings open and it could show, they are bound to
//! temporaries in order first.

use super::{Code, Fx, Ret, prims};
use crate::facts::{Effects, any_order};
use crate::names::{ident, string_literal};
use spec_lang_lir::node::*;
use spec_lang_lir::{LTy, Prim};
use std::fmt::Write;

impl<'g, 'a> Fx<'g, 'a> {
    /// `e` as a C++ expression.
    pub fn expr(&mut self, e: &Expr) -> Code {
        match &e.kind {
            ExprKind::Unit => Code::atom("rt::unit"),
            ExprKind::Bool(b) => Code::atom(if *b { "true" } else { "false" }),
            ExprKind::Int(n) => int_literal(*n),
            ExprKind::Text(t) => Code::atom(format!("{}s", string_literal(t))),
            ExprKind::Interp(parts) => self.interp(parts),
            ExprKind::Local(n) => Code::atom(self.local(n)),
            ExprKind::Const(c) => Code::atom(self.g.const_ref(*c)),
            ExprKind::State(s) => Code::atom(format!("{}.world.{}", self.ctx(e.span), self.g.state_field(*s))),
            ExprKind::Variant { ty, tag } => Code::atom(self.variant(*ty, *tag, "{}")),
            ExprKind::Some(x) => Code::atom(format!("rt::some({})", self.expr(x).at(2))),
            ExprKind::None => Code::atom(format!("{}{{}}", self.g.ty(&e.ty))),
            ExprKind::Construct { ty, tag, args } => self.construct(*ty, *tag, args),
            ExprKind::CallFn { func, args } => {
                let d = self.g.lir.func(*func);
                let lead = if self.g.facts.fn_ctx[func.index()] { Some(self.ctx(e.span)) } else { None };
                self.call(&self.g.fn_name(*func), lead, &d.params, args, &e.ty, d.result == LTy::Unit)
            }
            ExprKind::CallRule { rule, args } => {
                let d = self.g.lir.rule(*rule);
                let lead = Some(self.ctx(e.span));
                self.call(&self.g.rule_name(*rule), lead, &d.params, args, &e.ty, d.result == LTy::Unit)
            }
            ExprKind::Effect { handler, op, args, .. } => {
                let h = self.g.lir.handler(*handler);
                let o = &h.ops[*op as usize];
                let target = format!("{}.{}->{}", self.ctx(e.span), ident(&h.name), ident(&o.name));
                let refs: Vec<&Expr> = args.iter().collect();
                let void = o.result == LTy::Unit;
                self.ordered(&refs, &e.ty, |a| {
                    let call = format!("{target}({})", a.iter().map(|x| x.at(2)).collect::<Vec<_>>().join(", "));
                    if void { Code::new(format!("{call}, rt::unit"), 1) } else { Code::atom(call) }
                })
            }
            ExprKind::EffectRead { handler, op, .. } => {
                let h = self.g.lir.handler(*handler);
                Code::atom(format!("{}.{}->{}()", self.ctx(e.span), ident(&h.name), ident(&h.ops[*op as usize].name)))
            }
            ExprKind::Prim { op, args } => {
                let refs: Vec<&Expr> = args.iter().collect();
                self.ordered(&refs, &e.ty, |a| prims::apply(*op, a))
            }
            ExprKind::Lambda(l) => self.lambda(l, false),
            ExprKind::CallLambda { callee, args } => {
                let mut refs: Vec<&Expr> = vec![callee];
                refs.extend(args.iter());
                self.ordered(&refs, &e.ty, |a| {
                    Code::atom(format!(
                        "{}({})",
                        a[0].at(16),
                        a[1..].iter().map(|x| x.at(2)).collect::<Vec<_>>().join(", ")
                    ))
                })
            }
            ExprKind::Field { recv, ty, index, .. } => {
                Code::atom(self.g.field(&self.expr(recv).at(16), *ty, None, *index))
            }
            ExprKind::VariantField { recv, name, .. } => {
                Code::atom(format!("{}.{}()", self.expr(recv).at(16), ident(name)))
            }
            ExprKind::WorldState { recv, state } => {
                Code::atom(format!("{}.{}", self.expr(recv).at(16), self.g.state_field(*state)))
            }
            ExprKind::EventArg { recv, name, by_rule } => {
                let table: Vec<String> = by_rule
                    .iter()
                    .filter_map(|(r, i)| i.map(|i| format!("{{{}, {i}}}", self.g.rule_const(*r))))
                    .collect();
                Code::atom(format!(
                    "{}.arg<{}>({}, {{{}}})",
                    self.expr(recv).at(16),
                    self.g.ty(&e.ty),
                    string_literal(name),
                    table.join(", ")
                ))
            }
            ExprKind::With { recv, ty, changes } => self.with(recv, *ty, changes),
            ExprKind::Is { value, pat } => self.is(value, pat),
            ExprKind::If { cond, then, els } => {
                if needs_statements(e) {
                    return self.iife(e);
                }
                let c = self.expr(cond).at(3);
                Code::new(format!("{c} ? {} : {}", self.expr(then).at(3), self.expr(els).at(2)), 2)
            }
            ExprKind::Logic { op, left, right } => {
                let (l, r) = (self.expr(left), self.expr(right));
                match op {
                    Logic::And => Code::new(format!("{} && {}", l.at(4), r.at(5)), 4),
                    Logic::Or => Code::new(format!("{} || {}", l.at(5), r.at(5)), 3),
                    Logic::Implies => Code::new(format!("!{} || {}", l.at(15), r.at(5)), 3),
                }
            }
            ExprKind::Compare { first, links } => self.compare(e, first, links),
            ExprKind::Else { value, fallback, unwrap } => self.else_(value, fallback, *unwrap),
            ExprKind::Fail { name, args } => {
                let refs: Vec<&Expr> = args.iter().collect();
                let name = string_literal(name);
                self.ordered(&refs, &e.ty, |a| {
                    let mut all = vec![name.clone()];
                    all.extend(a.iter().map(|x| x.at(2)));
                    Code::atom(format!("rt::fail({})", all.join(", ")))
                })
            }
            ExprKind::Match { .. } | ExprKind::Block(_) => self.iife(e),
            ExprKind::Hole(h) => Code::atom(format!("rt::hole({})", string_literal(h))),
            ExprKind::Tuple(xs) | ExprKind::List(xs) | ExprKind::SetOf(xs) => {
                // A braced list is evaluated in order.
                let items: Vec<String> = xs.iter().map(|x| self.expr(x).at(2)).collect();
                Code::atom(format!("{}{{{}}}", self.g.ty(&e.ty), items.join(", ")))
            }
            ExprKind::MapOf(pairs) => {
                let items: Vec<String> =
                    pairs.iter().map(|(k, v)| format!("{{{}, {}}}", self.expr(k).at(2), self.expr(v).at(2))).collect();
                let distinct = pairs.iter().all(|(k, _)| matches!(k.kind, ExprKind::Int(_) | ExprKind::Text(_)))
                    && (1..pairs.len()).all(|i| pairs[..i].iter().all(|(k, _)| !same_literal(k, &pairs[i].0)));
                if distinct {
                    Code::atom(format!("{}{{{}}}", self.g.ty(&e.ty), items.join(", ")))
                } else {
                    Code::atom(format!("rt::map_of<{}>({{{}}})", self.g.ty(&e.ty), items.join(", ")))
                }
            }
            ExprKind::Range { lo, hi, inclusive } => {
                Code::atom(format!("rt::Range{{{}, {}, {inclusive}}}", self.expr(lo).at(2), self.expr(hi).at(2)))
            }
        }
    }

    /// A value of a type the context already names: an empty collection or `none` is `{}`.
    pub fn expr_typed(&mut self, e: &Expr) -> String {
        match &e.kind {
            ExprKind::None => "{}".into(),
            ExprKind::List(xs) | ExprKind::SetOf(xs) if xs.is_empty() => "{}".into(),
            ExprKind::MapOf(xs) if xs.is_empty() => "{}".into(),
            _ => self.expr(e).at(2),
        }
    }

    /// What evaluating `e` as a sibling does: making a lambda does nothing, whatever its body would.
    fn sibling(&self, e: &Expr) -> Effects {
        match &e.kind {
            ExprKind::Lambda(_) => Effects::default(),
            _ => self.g.facts.effects(e),
        }
    }

    fn arg(&mut self, e: &Expr) -> Code {
        match &e.kind {
            // A lambda handed straight to a call does not outlive it.
            ExprKind::Lambda(l) => self.lambda(l, true),
            _ => self.expr(e),
        }
    }

    /// Siblings in the language's order: plain when C++ may pick any order, else through temporaries.
    pub fn ordered(&mut self, parts: &[&Expr], ty: &LTy, build: impl FnOnce(&[Code]) -> Code) -> Code {
        let fx: Vec<Effects> = parts.iter().map(|p| self.sibling(p)).collect();
        if any_order(&fx) {
            let codes: Vec<Code> = parts.iter().map(|p| self.arg(p)).collect();
            return build(&codes);
        }
        let mut body = String::new();
        let mut codes = Vec::new();
        for (p, f) in parts.iter().zip(&fx) {
            let code = self.arg(p);
            if *f == Effects::default() || matches!(p.kind, ExprKind::Local(_)) && !fx.iter().any(|o| o.writes_local) {
                codes.push(code);
            } else {
                let name = self.fresh("arg");
                let _ = writeln!(body, "const auto {name} = {};", code.at(2));
                codes.push(Code::atom(name));
            }
        }
        let built = build(&codes);
        Code::atom(format!("[&]() -> {} {{\n{body}return {};\n}}()", self.g.ty(ty), built.at(2)))
    }

    fn interp(&mut self, parts: &[Part]) -> Code {
        let mut items = Vec::new();
        for p in parts {
            match p {
                Part::Lit(l) => items.push(string_literal(l)),
                Part::Show(x) if x.ty == LTy::Text => items.push(self.expr(x).at(2)),
                Part::Show(x) => items.push(format!("rt::str({})", self.expr(x).at(2))),
            }
        }
        match (parts, items.as_slice()) {
            ([], _) => Code::atom("rt::Text{}"),
            ([Part::Show(x)], [one]) if x.ty == LTy::Text => Code::atom(one.clone()),
            // A braced list is evaluated in order.
            _ => Code::atom(format!("rt::concat({{{}}})", items.join(", "))),
        }
    }

    /// A variant as a value; `fields` is its braced field list.
    fn variant(&self, ty: spec_lang_ir::TypeId, tag: u32, fields: &str) -> String {
        if self.g.is_enum(ty) {
            self.g.variant_name(ty, tag)
        } else {
            format!("{}{{{}{fields}}}", self.g.type_name(ty), self.g.variant_name(ty, tag))
        }
    }

    fn construct(&mut self, ty: spec_lang_ir::TypeId, tag: Option<u32>, args: &[Option<Expr>]) -> Code {
        let fields = self.g.fields(ty, tag);
        // Designated initializers run in field order; a field not given takes its default member initializer.
        let mut items = Vec::new();
        for (f, a) in fields.iter().zip(args) {
            if let Some(a) = a {
                items.push(format!(".{} = {}", ident(&f.name), self.expr_typed(a)));
            }
        }
        let braces = format!("{{{}}}", items.join(", "));
        let value = match tag {
            Some(tag) => self.variant(ty, tag, &braces),
            None => format!("{}{braces}", self.g.type_name(ty)),
        };
        if self.g.facts.type_checks[ty.index()] {
            let what = string_literal(&format!("a new {}", self.g.lir.ty(ty).info.name));
            Code::atom(format!("rt::conforming({value}, {}, {what})", self.g.valid_name(ty)))
        } else {
            Code::atom(value)
        }
    }

    /// A call of a `fn` or a rule. Trailing arguments not given use the C++ default argument; one in the
    /// middle is written out, with the earlier parameters in scope when its default reads them.
    fn call(
        &mut self,
        name: &str,
        lead: Option<String>,
        params: &[spec_lang_lir::Param],
        args: &[Option<Expr>],
        ty: &LTy,
        void: bool,
    ) -> Code {
        let given = args.iter().rposition(Option::is_some).map_or(0, |i| i + 1).max(self.g.first_cpp_default(params));
        let finish = |mut all: Vec<String>| {
            if let Some(l) = &lead {
                all.insert(0, l.clone());
            }
            let call = format!("{name}({})", all.join(", "));
            if void { Code::new(format!("{call}, rt::unit"), 1) } else { Code::atom(call) }
        };
        let reads_params = |d: &Expr| !super::closed(d);
        let scoped = (0..given).any(|i| args[i].is_none() && params[i].default.as_ref().is_some_and(reads_params));
        if !scoped {
            // Defaults are closed: each stands where its argument would.
            let exprs: Vec<&Expr> =
                (0..given).filter_map(|i| args[i].as_ref().or(params[i].default.as_ref())).collect();
            return self.ordered(&exprs, ty, |a| finish(a.iter().map(|x| x.at(2)).collect()));
        }
        // Given arguments first, in order; then each default with the parameters before it in scope.
        let mut body = String::new();
        let mut names = vec![String::new(); given];
        for i in 0..given {
            if let Some(a) = &args[i] {
                names[i] = self.fresh("arg");
                let _ = writeln!(body, "const auto {} = {};", names[i], self.expr(a).at(2));
            }
        }
        self.push_scope();
        for i in 0..given {
            if let (None, Some(d)) = (&args[i], &params[i].default) {
                names[i] = self.fresh("arg");
                let _ = writeln!(body, "const auto {} = {};", names[i], self.expr(d).at(2));
            }
            self.bind(&params[i].name, &names[i].clone());
        }
        self.pop_scope();
        Code::atom(format!("[&]() -> {} {{\n{body}return {};\n}}()", self.g.ty(ty), finish(names).at(2)))
    }

    /// A lambda. Captured locals are copied when it is made; one handed straight to a call may refer to them.
    fn lambda(&mut self, l: &Lambda, immediate: bool) -> Code {
        self.push_scope();
        let params: Vec<String> = l
            .params
            .iter()
            .map(|(n, t)| {
                let name = self.declare(n);
                format!("{}{} {name}", self.mark(&name), self.g.param_ty(t))
            })
            .collect();
        let outer_ctx = std::mem::replace(&mut self.ctx_used, false);
        let (body, explicit) = match &l.body {
            LambdaBody::Expr(x) if !needs_statements(x) => {
                (format!("return {};", self.expr(x).at(2)), x.ty != l.result || matches!(x.ty, LTy::Unit))
            }
            LambdaBody::Expr(x) => {
                let mut out = String::new();
                self.tail(x, &mut out, Ret::Value);
                (out, true)
            }
            LambdaBody::Block(b) => {
                let mut out = String::new();
                self.block(b, &mut out, Ret::Value);
                (out, true)
            }
        };
        self.pop_scope();
        let writes_capture = match &l.body {
            LambdaBody::Block(b) => self.g.facts.block(b).writes_local && !l.captures.is_empty(),
            LambdaBody::Expr(x) => self.g.facts.effects(x).writes_local && !l.captures.is_empty(),
        };
        let capture = if immediate && !writes_capture {
            "&"
        } else if self.ctx_used {
            "=, &ctx"
        } else {
            "="
        };
        if capture != "&" && self.ctx_used && self.ctx.as_deref() != Some("ctx") {
            self.g.unsupported(self.span, "a lambda that keeps a machine of another name");
        }
        self.ctx_used |= outer_ctx;
        let ret = if explicit { format!(" -> {}", self.g.ty(&l.result)) } else { String::new() };
        let mutable = if writes_capture { " mutable" } else { "" };
        Code::atom(format!("[{capture}]({}){mutable}{ret} {{\n{body}\n}}", params.join(", ")))
    }

    fn with(&mut self, recv: &Expr, ty: spec_lang_ir::TypeId, changes: &[(u32, Expr)]) -> Code {
        let fields = self.g.fields(ty, None);
        let copy = self.fresh("changed");
        let mut body = format!("auto {copy} = {};\n", self.expr(recv).at(2));
        let mut claims = String::new();
        for (i, x) in changes {
            let f = &fields[*i as usize];
            let _ = writeln!(body, "{copy}.{} = {};", ident(&f.name), self.expr_typed(x));
            if let Some(c) = self.conform(&f.check, &format!("{copy}.{}", ident(&f.name))) {
                let _ = writeln!(claims, "rt::claim({c}, {});", string_literal(&format!("field `{}`", f.name)));
            }
        }
        Code::atom(format!("[&] {{\n{body}{claims}return {copy};\n}}()"))
    }

    fn compare(&mut self, whole: &Expr, first: &Expr, links: &[(Prim, Expr)]) -> Code {
        let link = |op: Prim, left: Code, right: Code| {
            if op.is_containment() { prims::apply(op, &[right, left]) } else { prims::apply(op, &[left, right]) }
        };
        if let [(op, right)] = links {
            return self.ordered(&[first, right], &whole.ty, |a| link(*op, a[0].clone(), a[1].clone()));
        }
        // A chain: each right side runs once, and only while every earlier link held.
        let repeatable =
            |e: &Expr| matches!(e.kind, ExprKind::Local(_) | ExprKind::Int(_) | ExprKind::Text(_) | ExprKind::Const(_));
        let mut body = String::new();
        let mut left = self.expr(first);
        if !repeatable(first) {
            let name = self.fresh("left");
            let _ = writeln!(body, "const auto {name} = {};", left.at(2));
            left = Code::atom(name);
        }
        let all_plain = body.is_empty() && links.iter().all(|(_, x)| repeatable(x));
        if all_plain {
            let mut terms = Vec::new();
            for (op, x) in links {
                let right = self.expr(x);
                terms.push(link(*op, left, right.clone()).at(5));
                left = right;
            }
            return Code::new(terms.join(" && "), 4);
        }
        for (op, x) in links {
            let name = self.fresh("right");
            let _ = writeln!(body, "const auto {name} = {};", self.expr(x).at(2));
            let _ = writeln!(body, "if (!({})) return false;", link(*op, left, Code::atom(name.clone())).s);
            left = Code::atom(name);
        }
        Code::atom(format!("[&]() -> bool {{\n{body}return true;\n}}()"))
    }

    fn else_(&mut self, value: &Expr, fallback: &Expr, unwrap: bool) -> Code {
        let fx = self.g.facts.effects(value);
        let v = self.expr(value);
        if !unwrap && !fx.can_fail() {
            return v;
        }
        let simple = |e: &Expr| {
            matches!(
                e.kind,
                ExprKind::Local(_)
                    | ExprKind::Int(_)
                    | ExprKind::Text(_)
                    | ExprKind::Bool(_)
                    | ExprKind::Const(_)
                    | ExprKind::Variant { .. }
            )
        };
        if unwrap && !fx.can_fail() && simple(fallback) {
            return Code::atom(format!("{}.value_or({})", v.at(16), self.expr(fallback).at(2)));
        }
        let f = self.thunk(fallback);
        let name = if unwrap { "unwrap_or" } else { "or_else" };
        Code::atom(format!("rt::{name}([&] {{ return {}; }}, {f})", v.at(2)))
    }

    /// `[&] { return e; }`, or with statements when `e` needs them.
    pub fn thunk(&mut self, e: &Expr) -> String {
        if needs_statements(e) {
            let mut out = String::new();
            self.push_scope();
            self.tail(e, &mut out, Ret::Value);
            self.pop_scope();
            format!("[&]() -> {} {{\n{out}\n}}", self.g.ty(&e.ty))
        } else if matches!(e.ty, LTy::Unit) {
            format!("[&]() -> rt::Unit {{ return {}; }}", self.expr(e).at(2))
        } else {
            format!("[&] {{ return {}; }}", self.expr(e).at(2))
        }
    }

    /// An expression that needs statements, as a lambda called at once.
    fn iife(&mut self, e: &Expr) -> Code {
        let mut out = String::new();
        self.push_scope();
        self.tail(e, &mut out, Ret::Value);
        self.pop_scope();
        Code::atom(format!("[&]() -> {} {{\n{out}\n}}()", self.g.ty(&e.ty)))
    }
}

fn same_literal(a: &Expr, b: &Expr) -> bool {
    match (&a.kind, &b.kind) {
        (ExprKind::Int(x), ExprKind::Int(y)) => x == y,
        (ExprKind::Text(x), ExprKind::Text(y)) => x == y,
        _ => false,
    }
}

/// An Int literal: `5_i`, of any size that fits 128 bits.
pub fn int_literal(n: i128) -> Code {
    if n == i128::MIN {
        Code::atom("rt::Int::min()")
    } else if n < 0 {
        Code::new(format!("-{}_i", n.unsigned_abs()), 15)
    } else {
        Code::atom(format!("{n}_i"))
    }
}

/// Whether C++ needs statements for `e`: a block, a match, or an `if` with one of them in a branch.
pub fn needs_statements(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Block(_) | ExprKind::Match { .. } => true,
        ExprKind::If { then, els, .. } => needs_statements(then) || needs_statements(els),
        _ => false,
    }
}
