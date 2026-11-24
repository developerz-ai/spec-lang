//! Expressions: every `ExprKind` as Rust, in the evaluation order `src/crates/lir/README.md` gives.

use crate::code::{Code, Fx, Target};
use crate::names::{camel, ident, local, screaming};
use spec_lang_lir::node::{Expr, ExprKind, Lambda, LambdaBody, Logic, Part};
use spec_lang_lir::{LTy, TypeDef};

impl Fx<'_, '_> {
    /// An expression's owned value.
    pub fn value(&mut self, e: &Expr) -> Code {
        let c = self.expr(e);
        self.owned(c, &e.ty)
    }

    /// Owned values of arguments, each `Some(x)`/`None` where the parameter has a default.
    fn call_args(&mut self, params: &[(bool, String)], args: &[Option<Expr>], span: spec_lang_syntax::Span) -> Vec<Code> {
        let mut out = Vec::new();
        for ((has_default, name), a) in params.iter().zip(args) {
            out.push(match (a, has_default) {
                (Some(e), false) => self.value(e),
                (Some(e), true) => {
                    let c = self.value(e);
                    Code { text: format!("Some({})", c.text), atomic: true, ..c }
                }
                (None, true) => Code::lit("None"),
                (None, false) => {
                    self.cx.unsupported(span, format!("a call without argument `{name}`, which has no default"));
                    Code::lit("None")
                }
            });
        }
        out
    }

    /// An item of module `of`, as a path from here; `self::` when a local of the body has its name.
    fn item(&self, of: spec_lang_ir::ModuleId, name: &str) -> String {
        if self.module == Some(of) && self.locals.contains(name) {
            return format!("self::{name}");
        }
        self.cx.names.path(self.module, of, name)
    }

    pub fn expr(&mut self, e: &Expr) -> Code {
        let lir = self.cx.lir;
        match &e.kind {
            ExprKind::Unit => Code::lit("()"),
            ExprKind::Bool(b) => Code::lit(b.to_string()),
            ExprKind::Int(n) => int_lit(*n),
            ExprKind::Text(t) => Code::lit(format!("String::from({t:?})")),
            ExprKind::Interp(parts) => self.interp(parts),
            ExprKind::Local(n) => {
                Code { place: true, copy: self.cx.copy(&e.ty), ..Code::lit(local(n)) }
            }
            ExprKind::Const(c) => {
                let d = lir.constant(*c);
                if self.cx.const_literal(*c) {
                    let p = self.item(d.info.module, &screaming(&d.info.name));
                    return if matches!(d.value.kind, ExprKind::Text(_)) { Code::val(format!("{p}.to_string()")) } else { Code::lit(p) };
                }
                let p = self.item(d.info.module, &ident(&d.info.name));
                let m = self.cx.const_m[c.index()];
                self.call(p, m, Vec::new())
            }
            ExprKind::State(s) => Code {
                text: format!("m.world.{}", self.cx.names.state(*s)),
                place: true,
                atomic: true,
                m: true,
                copy: self.cx.copy(&e.ty),
                ..Code::default()
            },
            ExprKind::Variant { ty, tag } => {
                Code::lit(format!("{}::{}", self.cx.type_path(*ty, self.module), self.variant_name(*ty, *tag)))
            }
            ExprKind::Some(x) => {
                let c = match (&x.kind, &x.ty) {
                    (ExprKind::None, LTy::Option(t)) => Code::lit(format!("Opt::<{}>::None", self.ty(t))),
                    _ => self.value(x),
                };
                Code { text: format!("Opt::Some({})", c.text), atomic: true, simple: false, ..c }
            }
            ExprKind::None => Code::lit("Opt::None"),
            ExprKind::Construct { ty, tag, args } => self.construct(*ty, *tag, args, e.span),
            ExprKind::CallFn { func, args } => {
                let f = lir.func(*func);
                let params: Vec<(bool, String)> = f.params.iter().map(|p| (p.default.is_some(), p.name.clone())).collect();
                let args = self.call_args(&params, args, e.span);
                let p = self.item(f.info.module, &ident(&f.info.name));
                let m = self.cx.fn_m[func.index()];
                self.call(p, m, args)
            }
            ExprKind::CallRule { rule, args } => {
                let r = lir.rule(*rule);
                let params: Vec<(bool, String)> = r.params.iter().map(|p| (p.default.is_some(), p.name.clone())).collect();
                let args = self.call_args(&params, args, e.span);
                let p = self.item(r.info.module, &ident(&r.info.name));
                self.call(p, true, args)
            }
            ExprKind::Effect { handler, op, args, .. } => {
                let args = args.iter().map(|a| self.value(a)).collect();
                let p = format!("crate::machine::{}", crate::machine::op_fn(lir, *handler, *op));
                self.call(p, true, args)
            }
            ExprKind::EffectRead { handler, op, .. } => {
                let p = format!("crate::machine::{}", crate::machine::op_fn(lir, *handler, *op));
                self.call(p, true, Vec::new())
            }
            ExprKind::Prim { op, args } => self.prim(*op, args, &e.ty),
            ExprKind::Lambda(l) => self.lambda_value(l),
            ExprKind::CallLambda { callee, args } => {
                let mut items = vec![self.expr(callee)];
                items.extend(args.iter().map(|a| self.value(a)));
                let (pre, items) = self.args(items, true);
                let tuple = tuple_text(items[1..].iter().map(|c| c.text.clone()).collect());
                Code { pre, text: format!("{}.call(m, {tuple})?", items[0].operand()), atomic: true, m: true, fail: true, ..Code::default() }
            }
            ExprKind::Field { recv, ty, index, name } => {
                let r = self.expr(recv);
                let boxed = self.cx.is_boxed(*ty, crate::cx::RECORD, *index);
                let f = ident(name);
                let text = if boxed { format!("(*{}.{f})", r.operand()) } else { format!("{}.{f}", r.operand()) };
                Code { text, atomic: true, simple: r.simple && r.place, copy: self.cx.copy(&e.ty), ..r }
            }
            ExprKind::VariantField { recv, ty, name, by_tag } => {
                let r = self.expr(recv);
                let path = self.cx.type_path(*ty, self.module);
                let mut arms = Vec::new();
                let all = by_tag.iter().all(Option::is_some);
                for (tag, i) in by_tag.iter().enumerate() {
                    let Some(i) = i else { continue };
                    let boxed = self.cx.is_boxed(*ty, tag as u32, *i);
                    let read = if boxed { "(**x).clone()".to_string() } else if self.cx.copy(&e.ty) { "*x".into() } else { "x.clone()".into() };
                    arms.push(format!("{path}::{} {{ {}: x, .. }} => {read},", self.variant_name(*ty, tag as u32), ident(name)));
                }
                if !all {
                    arms.push(format!("_ => return Err(rt::stop(\"E-TYPE-MISMATCH\", \"this variant has no `{name}`\")),"));
                }
                let fail = !all;
                Code { pre: r.pre.clone(), text: format!("match {} {{ {} }}", r.reference(), arms.join(" ")), atomic: true, m: r.m, fail: r.fail || fail, ..Code::default() }
            }
            ExprKind::WorldState { recv, state } => {
                let r = self.expr(recv);
                let text = format!("{}.{}", r.operand(), self.cx.names.state(*state));
                Code { text, atomic: true, copy: self.cx.copy(&e.ty), ..r }
            }
            ExprKind::EventArg { recv, name, by_rule } => {
                let r = self.expr(recv);
                let mut arms = Vec::new();
                for (rule, i) in by_rule {
                    if i.is_none() {
                        continue;
                    }
                    let read = if self.cx.copy(&e.ty) { "*x" } else { "x.clone()" };
                    arms.push(format!("crate::machine::Event::{} {{ {}: x, .. }} => {read},", self.cx.names.event(*rule), ident(name)));
                }
                arms.push(format!("#[allow(unreachable_patterns)] _ => return Err(rt::stop(\"E-TYPE-MISMATCH\", \"the event has no argument `{name}`\")),"));
                Code { pre: r.pre.clone(), text: format!("match {} {{ {} }}", r.reference(), arms.join(" ")), atomic: true, m: r.m, fail: true, ..Code::default() }
            }
            ExprKind::With { recv, ty, changes } => self.with(recv, *ty, changes),
            ExprKind::Is { value, pat } => self.is(value, pat),
            ExprKind::If { cond, then, els } => {
                let c = self.value(cond);
                let t = self.value(then);
                let f = self.value(els);
                Code { pre: c.pre.clone(), text: format!("if {} {{ {} }} else {{ {} }}", c.text, t.seal(), f.seal()), ..Code::op("") }
                    .with(&[&c, &t, &f])
            }
            ExprKind::Logic { op, left, right } => {
                let l = self.value(left);
                let r = self.value(right).sealed();
                let text = match op {
                    Logic::And => format!("{} && {}", l.operand(), r.operand()),
                    Logic::Or => format!("{} || {}", l.operand(), r.operand()),
                    Logic::Implies => format!("!{} || {}", l.operand(), r.operand()),
                };
                Code { pre: l.pre.clone(), ..Code::op(text) }.with(&[&l, &r])
            }
            ExprKind::Compare { first, links } => self.compare(first, links),
            ExprKind::Else { value, fallback, unwrap } => self.else_(value, fallback, *unwrap),
            ExprKind::Fail { name, args } => {
                let items = args.iter().map(|a| self.value(a)).collect();
                let (pre, items) = self.args(items, false);
                let data: Vec<String> = items.iter().map(|c| format!("{}.show()", c.operand())).collect();
                let m = items.iter().any(|c| c.m);
                Code { pre, text: format!("return Err(rt::fail({name:?}, vec![{}]))", data.join(", ")), atomic: false, m, fail: true, ..Code::default() }
            }
            ExprKind::Match { scrutinee, arms } => self.match_(scrutinee, arms, &e.ty),
            ExprKind::Block(b) => self.block_value(b),
            ExprKind::Hole(h) => Code { text: format!("return Err(rt::hole({h:?}))"), fail: true, ..Code::op("") },
            ExprKind::Tuple(xs) => {
                let items = xs.iter().map(|x| self.value(x)).collect();
                let (pre, items) = self.args(items, false);
                Code { pre, text: tuple_text(items.iter().map(|c| c.text.clone()).collect()), atomic: true, ..Code::default() }
                    .with(&items.iter().collect::<Vec<_>>())
            }
            ExprKind::List(xs) => self.collection(xs, &e.ty, "vec![{}]", "Vec::<{}>::new()"),
            ExprKind::SetOf(xs) => self.collection(xs, &e.ty, "BTreeSet::from([{}])", "BTreeSet::<{}>::new()"),
            ExprKind::MapOf(pairs) => {
                if pairs.is_empty() {
                    let t = match &e.ty {
                        LTy::Map(k, v) => format!("{}, {}", self.ty(k), self.ty(v)),
                        _ => "(), ()".into(),
                    };
                    return Code::val(format!("BTreeMap::<{t}>::new()"));
                }
                let mut items = Vec::new();
                for (k, v) in pairs {
                    items.push(self.value(k));
                    items.push(self.value(v));
                }
                let (pre, items) = self.args(items, false);
                let entries: Vec<String> =
                    items.chunks(2).map(|kv| format!("({}, {})", kv[0].text, kv[1].text)).collect();
                Code { pre, text: format!("rt::map_of(vec![{}])", entries.join(", ")), atomic: true, ..Code::default() }
                    .with(&items.iter().collect::<Vec<_>>())
            }
            ExprKind::Range { lo, hi, inclusive } => {
                let items = vec![self.value(lo), self.value(hi)];
                let (pre, items) = self.args(items, false);
                Code {
                    pre,
                    text: format!("Range {{ lo: {}, hi: {}, inclusive: {inclusive} }}", items[0].text, items[1].text),
                    atomic: false,
                    ..Code::default()
                }
                .with(&items.iter().collect::<Vec<_>>())
            }
        }
    }

    pub fn variant_name(&self, ty: spec_lang_ir::TypeId, tag: u32) -> String {
        match &self.cx.lir.ty(ty).def {
            TypeDef::Sum { variants } => variants.get(tag as usize).map_or_else(|| "Unknown".into(), |v| ident(&camel(&v.name))),
            _ => "Unknown".into(),
        }
    }

    fn collection(&mut self, xs: &[Expr], ty: &LTy, full: &str, empty: &str) -> Code {
        if xs.is_empty() {
            let item = match ty {
                LTy::List(t) | LTy::Set(t) => self.ty(t),
                _ => "()".into(),
            };
            return Code::val(empty.replace("{}", &item));
        }
        let items = xs.iter().map(|x| self.value(x)).collect();
        let (pre, items) = self.args(items, false);
        let list: Vec<String> = items.iter().map(|c| c.text.clone()).collect();
        Code { pre, text: full.replace("{}", &list.join(", ")), atomic: true, ..Code::default() }
            .with(&items.iter().collect::<Vec<_>>())
    }

    fn interp(&mut self, parts: &[Part]) -> Code {
        let mut fmt = String::new();
        let mut items = Vec::new();
        for p in parts {
            match p {
                Part::Lit(l) => fmt.push_str(&l.replace('{', "{{").replace('}', "}}")),
                Part::Show(x) => match &x.kind {
                    ExprKind::Int(n) => fmt.push_str(&n.to_string()),
                    _ => {
                        fmt.push_str("{}");
                        let c = self.expr(x);
                        let c = if x.ty == LTy::Text {
                            c
                        } else {
                            Code { text: format!("{}.show()", c.operand()), atomic: true, place: false, simple: false, ..c }
                        };
                        items.push(c);
                    }
                },
            }
        }
        if items.is_empty() {
            return Code::lit(format!("String::from({fmt:?})"));
        }
        let (pre, items) = self.args(items, false);
        let args: Vec<String> = items.iter().map(|c| c.text.clone()).collect();
        Code { pre, text: format!("format!({fmt:?}, {})", args.join(", ")), atomic: true, ..Code::default() }
            .with(&items.iter().collect::<Vec<_>>())
    }

    fn construct(&mut self, ty: spec_lang_ir::TypeId, tag: Option<u32>, args: &[Option<Expr>], span: spec_lang_syntax::Span) -> Code {
        let lir = self.cx.lir;
        let fields = match (&lir.ty(ty).def, tag) {
            (TypeDef::Record { fields }, None) => fields,
            (TypeDef::Sum { variants }, Some(t)) => match variants.get(t as usize) {
                Some(v) => &v.fields,
                None => {
                    self.cx.unsupported(span, "a constructor of no variant");
                    return Code::lit("()");
                }
            },
            _ => {
                self.cx.unsupported(span, "a constructor of a type that is neither a record nor a sum");
                return Code::lit("()");
            }
        };
        let path = self.cx.type_path(ty, self.module);
        let variant = tag.map(|t| self.variant_name(ty, t));
        if crate::decls::needs_new(self.cx, fields) {
            let params: Vec<(bool, String)> = fields.iter().map(|f| (f.default.is_some(), f.name.clone())).collect();
            let args = self.call_args(&params, args, span);
            let m = crate::decls::new_m(self.cx, fields);
            let ctor = match &variant {
                Some(v) => format!("{path}::{}", crate::decls::ctor_name(v)),
                None => format!("{path}::new"),
            };
            return self.call(ctor, m, args);
        }
        let mut items = Vec::new();
        for a in args {
            match a {
                Some(e) => items.push(self.value(e)),
                None => {
                    self.cx.unsupported(span, "a field left out that has no default");
                    items.push(Code::lit("()"));
                }
            }
        }
        let (pre, items) = self.args(items, false);
        let inits: Vec<String> = fields
            .iter()
            .zip(&items)
            .enumerate()
            .map(|(i, (f, c))| {
                let boxed = self.cx.is_boxed(ty, tag.unwrap_or(crate::cx::RECORD), i as u32);
                let v = if boxed { format!("Box::new({})", c.text) } else { c.text.clone() };
                let n = ident(&f.name);
                if v == n { n } else { format!("{n}: {v}") }
            })
            .collect();
        let head = match variant {
            Some(v) => format!("{path}::{v}"),
            None => path,
        };
        let literal = !inits.is_empty();
        let text = if literal { format!("{head} {{ {} }}", inits.join(", ")) } else { head };
        // A struct literal is not an operand where a block may follow (`for`, `match`, `if`).
        Code { pre, text, atomic: !literal, ..Code::default() }.with(&items.iter().collect::<Vec<_>>())
    }

    fn with(&mut self, recv: &Expr, ty: spec_lang_ir::TypeId, changes: &[(u32, Expr)]) -> Code {
        let lir = self.cx.lir;
        let TypeDef::Record { fields } = &lir.ty(ty).def else {
            self.cx.unsupported(recv.span, "`.with` on a type that is not a record");
            return Code::lit("()");
        };
        let mut items = vec![self.value(recv)];
        items.extend(changes.iter().map(|(_, x)| self.value(x)));
        let (mut pre, items) = self.args(items, false);
        let any_m = items.iter().any(|c| c.m);
        let t = self.tmp();
        let mut body = vec![format!("let mut {t} = {};", items[0].text)];
        let mut fail = items.iter().any(|c| c.fail);
        let mut m = any_m;
        for ((i, _), c) in changes.iter().zip(&items[1..]) {
            let f = &fields[*i as usize];
            let v = self.tmp();
            body.push(format!("let {v} = {};", c.text));
            if let Some(check) = self.check(&f.check, &v, &format!("field `{}`", f.name)) {
                body.push(check);
                fail = true;
                m |= self.cx.dty_m(&f.check);
            }
            let boxed = self.cx.is_boxed(ty, crate::cx::RECORD, *i);
            let v = if boxed { format!("Box::new({v})") } else { v };
            body.push(format!("{t}.{} = {v};", ident(&f.name)));
        }
        pre.append(&mut body);
        Code { pre, text: t, atomic: true, m, fail, ..Code::default() }
    }

    /// A statement checking `v` (a reference-able name) against `d`, or nothing when the check cannot fail.
    pub fn check(&mut self, d: &spec_lang_lir::DTy, v: &str, what: &str) -> Option<String> {
        if self.cx.trivial(d) {
            return None;
        }
        let f = crate::dty::conform_fn(self.cx, d);
        let m = if self.cx.dty_m(d) { "m, " } else { "" };
        Some(format!("rt::claim(crate::conform::{f}({m}&{v})?, {what:?})?;"))
    }

    fn compare(&mut self, first: &Expr, links: &[(spec_lang_lir::Prim, Expr)]) -> Code {
        // One link: the primitive itself. More: each right side runs only while the earlier links held.
        if let [(op, right)] = links {
            let args = if op.is_containment() { vec![right.clone(), first.clone()] } else { vec![first.clone(), right.clone()] };
            return self.prim(*op, &args, &LTy::Bool);
        }
        let l = self.value(first);
        let lt = self.tmp();
        let mut pre = l.pre.clone();
        pre.push(format!("let {lt} = {};", l.text));
        let mut left = (lt, first.ty.clone());
        let mut conds = Vec::new();
        let mut m = l.m;
        for (op, right) in links {
            let r = self.value(right);
            let rt = self.tmp();
            let mut inner = r.pre.clone();
            inner.push(format!("let {rt} = {};", r.text));
            let lhs = Code { place: true, copy: self.cx.copy(&left.1), ..Code::lit(left.0.clone()) };
            let rhs = Code { place: true, copy: self.cx.copy(&right.ty), ..Code::lit(rt.clone()) };
            let (a, b) = if op.is_containment() { ((rhs, right.ty.clone()), (lhs, left.1.clone())) } else { ((lhs, left.1.clone()), (rhs, right.ty.clone())) };
            let held = self.prim_on(*op, vec![a, b], &LTy::Bool);
            m |= r.m || held.m;
            conds.push((inner, held.seal()));
            left = (rt, right.ty.clone());
        }
        // a && { let r2 = ..; op(r1, r2) } && ...: built from the last link outwards.
        let mut text = String::new();
        for (i, (inner, held)) in conds.iter().enumerate().rev() {
            text = if i + 1 == conds.len() {
                format!("{{ {} {held} }}", inner.join(" "))
            } else {
                format!("{{ {} {held} && {text} }}", inner.join(" "))
            };
        }
        Code { pre, text, atomic: true, m, fail: true, ..Code::default() }
    }

    fn else_(&mut self, value: &Expr, fallback: &Expr, unwrap: bool) -> Code {
        let v = self.value(value);
        let f = self.value(fallback);
        let is_opt = unwrap && matches!(value.ty, LTy::Option(_));
        if !v.fail && !is_opt {
            return v;
        }
        let fb = f.seal();
        let text = match (v.fail, is_opt) {
            (false, true) => format!("match {} {{ Opt::Some(x) => x, Opt::None => {fb} }}", v.seal()),
            (true, true) => format!("match rt::handled(rt::catch(|| {}))? {{ Some(Opt::Some(x)) => x, _ => {fb} }}", crate::code::ok(&v.seal())),
            _ => format!("match rt::handled(rt::catch(|| {}))? {{ Some(x) => x, None => {fb} }}", crate::code::ok(&v.seal())),
        };
        Code { text, atomic: true, m: v.m || f.m, fail: true, ..Code::default() }
    }

    /// A lambda as a value: its captured locals copied now.
    fn lambda_value(&mut self, l: &Lambda) -> Code {
        let params: Vec<String> = l.params.iter().map(|(n, _)| local(n)).collect();
        let tys: Vec<LTy> = l.params.iter().map(|(_, t)| t.clone()).collect();
        let pat = tuple_text(params);
        let pty = self.cx.tuple_ty(&tys, self.module);
        let body = self.lambda_body(l, true);
        let head = format!("Fun::new(move |m: &mut crate::machine::Machine, {pat}: {pty}| -> R<{}> {{ {body} }})", self.ty(&l.result));
        if l.captures.is_empty() {
            return Code::val(head);
        }
        let copies: Vec<String> = l
            .captures
            .iter()
            .filter(|(_, t)| !self.cx.copy(t))
            .map(|(n, _)| format!("let {0} = {0}.clone();", local(n)))
            .collect();
        if copies.is_empty() {
            return Code::val(head);
        }
        Code::val(format!("{{ {} {head} }}", copies.join(" ")))
    }

    /// A lambda written where a primitive calls it: a closure over the body's locals, `m` included.
    pub fn lambda_inline(&mut self, l: &Lambda) -> Code {
        let params: Vec<String> =
            l.params.iter().map(|(n, t)| format!("{}: {}", local(n), self.ty(t))).collect();
        let m = self.cx.lambda_m(l);
        let body = self.lambda_body(l, false);
        Code { text: format!("|{}| -> R<{}> {{ {body} }}", params.join(", "), self.ty(&l.result)), atomic: false, m, simple: true, ..Code::default() }
    }

    fn lambda_body(&mut self, l: &Lambda, own_m: bool) -> String {
        let saved = (std::mem::replace(&mut self.targets, vec![Target::Fn]), self.has_m, std::mem::replace(&mut self.discard, false));
        if own_m {
            self.has_m = true;
        }
        // A capture the body assigns is the body's own copy.
        let assigned = crate::stmt::assigned_in_lambda(l);
        let mut out: Vec<String> = l
            .captures
            .iter()
            .filter(|(n, _)| assigned.contains(n))
            .map(|(n, _)| format!("let mut {0} = {0}.clone();", local(n)))
            .collect();
        match &l.body {
            LambdaBody::Expr(x) => {
                let c = self.value(x);
                out.extend(c.pre.iter().cloned());
                out.push(crate::code::ok(&c.text));
            }
            LambdaBody::Block(b) => out.extend(self.fn_body(b, &l.result)),
        }
        (self.targets, self.has_m, self.discard) = saved;
        out.join(" ")
    }
}

/// `(a,)`, `(a, b)`, `()`.
pub fn tuple_text(items: Vec<String>) -> String {
    match items.len() {
        0 => "()".into(),
        1 => format!("({},)", items[0]),
        _ => format!("({})", items.join(", ")),
    }
}

pub fn int_lit(n: i128) -> Code {
    if n == i128::MIN {
        return Code::lit("Int::MIN");
    }
    let digits = n.unsigned_abs().to_string();
    let text = if n.unsigned_abs() > i32::MAX as u128 { format!("{digits}_i128") } else { digits };
    if n < 0 { Code { atomic: false, ..Code::lit(format!("-{text}")) } } else { Code::lit(text) }
}
