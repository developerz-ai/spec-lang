//! Statements, blocks, places. A `return` leaves the innermost block expression: the Rust function or closure
//! when that block is its body (`return Ok(v)`), else a labelled block (`break 'b v`).

use crate::code::{Code, Fx, Target, declared, uses};
use crate::names::{ident, local};
use spec_lang_lir::node::{Block, Expected, ExprKind, Lambda, LambdaBody, Place, PlaceRoot, PlaceStep, Stmt, StmtKind};
use spec_lang_lir::{LTy, Prim, TypeDef, walk};
use std::collections::BTreeSet;

impl Fx<'_, '_> {
    /// A block expression's value.
    pub fn block_value(&mut self, b: &Block) -> Code {
        let mut out = Vec::new();
        let text = if nested_return(&b.stmts) {
            let l = self.label();
            self.targets.push(Target::Label(l.clone()));
            let tail = self.stmts(&b.stmts, &mut out);
            self.targets.pop();
            let tail = self.tail(tail, &b.stmts, &b.ty);
            format!("{l}: {{ {} {tail} }}", out.join(" "))
        } else {
            let tail = self.stmts(&b.stmts, &mut out);
            let tail = self.tail(tail, &b.stmts, &b.ty);
            format!("{{ {} {tail} }}", out.join(" "))
        };
        guess(text, self.has_m)
    }

    /// The body of a Rust function or closure returning `R<ty>`: statements ending in `Ok(..)`.
    pub fn fn_body(&mut self, b: &Block, ty: &LTy) -> Vec<String> {
        let mut out = Vec::new();
        self.targets.push(Target::Fn);
        let tail = self.stmts(&b.stmts, &mut out);
        self.targets.pop();
        match tail {
            Some(c) if self.discard => {
                out.push(format!("let _ = {};", c.text));
                out.push("Ok(())".into());
            }
            Some(c) => out.push(crate::code::ok(&c.text)),
            None if diverges(&b.stmts) => {}
            None if matches!(ty, LTy::Unit | LTy::Never) => out.push("Ok(())".into()),
            None => out.push(no_value()),
        }
        out
    }

    /// The value a block ends with: its last top-level `return`, else `()`.
    fn tail(&mut self, tail: Option<Code>, stmts: &[Stmt], ty: &LTy) -> String {
        match tail {
            Some(c) => c.text,
            None if diverges(stmts) => String::new(),
            None if matches!(ty, LTy::Unit | LTy::Never) => String::new(),
            None => no_value(),
        }
    }

    /// Prints statements up to the first top-level `return`, whose value it gives back.
    fn stmts(&mut self, stmts: &[Stmt], out: &mut Vec<String>) -> Option<Code> {
        let mut lets = Vec::new();
        let mut tail = None;
        for s in stmts {
            if let StmtKind::Let { name, .. } = &s.kind {
                let from = out.len();
                self.stmt(s, out);
                let n = local(name);
                let head = [format!("let {n}:"), format!("let mut {n}:")];
                if let Some(i) = (from..out.len()).find(|i| head.iter().any(|h| out[*i].starts_with(h))) {
                    lets.push((i, n));
                }
                continue;
            }
            if let StmtKind::Return(v) = &s.kind {
                let c = match v {
                    Some(e) => self.value(e),
                    None => Code::lit("()"),
                };
                out.extend(c.pre.iter().cloned());
                tail = Some(c);
                break;
            }
            self.stmt(s, out);
        }
        // A local nothing reads is declared so.
        for (i, n) in lets {
            let read = out[i + 1..].iter().any(|l| uses(l, &n)) || tail.as_ref().is_some_and(|c| uses(&c.text, &n));
            if !read {
                out[i] = out[i].replacen(&n, &format!("_{}", n.trim_start_matches("r#")), 1);
            }
        }
        tail
    }

    fn nested(&mut self, b: &Block) -> String {
        let mut out = Vec::new();
        if let Some(c) = self.stmts(&b.stmts, &mut out) {
            out.push(self.ret(&c.text));
        }
        out.join(" ")
    }

    fn ret(&self, v: &str) -> String {
        match self.targets.last() {
            Some(Target::Label(l)) => format!("break {l} {v};"),
            _ if self.discard => format!("{{ let _ = {v}; return Ok(()); }}"),
            _ => format!("return Ok({v});"),
        }
    }

    fn stmt(&mut self, s: &Stmt, out: &mut Vec<String>) {
        match &s.kind {
            StmtKind::Let { name, mutable, ty, check, value } => {
                let c = self.value(value);
                out.extend(c.pre.iter().cloned());
                let n = local(name);
                let mt = if *mutable && self.assigned.contains(name) { "mut " } else { "" };
                if matches!(ty, LTy::Never) {
                    out.push(format!("{};", c.text));
                    return;
                }
                out.push(format!("let {mt}{n}: {} = {};", self.ty(ty), c.text));
                if let Some(d) = check
                    && let Some(line) = self.check(d, &n, &format!("`{name}`"))
                {
                    out.push(line);
                }
            }
            StmtKind::Assign { place, value } => {
                let v = self.value(value);
                self.assign(place, v, out);
            }
            StmtKind::Mutate { place, op, args } => self.mutate(place, *op, args, out),
            StmtKind::Expr(e) => {
                let c = self.expr(e);
                out.extend(c.pre.iter().cloned());
                if matches!(e.ty, LTy::Unit | LTy::Never) || c.text.ends_with('?') {
                    out.push(format!("{};", c.text));
                } else {
                    out.push(format!("let _ = {};", c.text));
                }
            }
            StmtKind::Return(v) => {
                let c = match v {
                    Some(e) => self.value(e),
                    None => Code::lit("()"),
                };
                out.extend(c.pre.iter().cloned());
                out.push(self.ret(&c.text));
            }
            StmtKind::Assert(e) => {
                let c = self.value(e);
                out.extend(c.pre.iter().cloned());
                out.push(format!("rt::assert({})?;", c.text));
            }
            StmtKind::For { var, iter, body, .. } => {
                let c = self.value(iter);
                out.extend(c.pre.iter().cloned());
                let body = self.nested(body);
                out.push(format!("for {} in {} {{ {body} }}", declared(&local(var), &body), c.text));
            }
            StmtKind::While { cond, body } => {
                let c = self.value(cond);
                let body = self.nested(body);
                out.push(format!("while {} {{ {body} }}", c.seal()));
            }
            StmtKind::If { cond, then, els } => {
                let c = self.value(cond);
                out.extend(c.pre.iter().cloned());
                let t = self.nested(then);
                match els {
                    Some(b) => {
                        let f = self.nested(b);
                        out.push(format!("if {} {{ {t} }} else {{ {f} }}", c.text));
                    }
                    None => out.push(format!("if {} {{ {t} }}", c.text)),
                }
            }
            StmtKind::Expect { subject, expect } => {
                let sub = self.value(subject);
                let got = self.tmp();
                out.push(format!("let {got} = rt::settle(rt::catch(|| {}))?;", crate::code::ok(&sub.seal())));
                match expect {
                    Expected::Value(e) => {
                        let w = self.value(e);
                        out.extend(w.pre.iter().cloned());
                        out.push(format!("rt::expect_value({got}, {})?;", w.text));
                    }
                    Expected::Fail { name, args } => {
                        let items = args.iter().map(|a| self.value(a)).collect();
                        let (pre, items) = self.args(items, false);
                        out.extend(pre);
                        let data: Vec<String> = items.iter().map(|c| format!("{}.show()", c.operand())).collect();
                        out.push(format!("rt::expect_fail({got}, {name:?}, vec![{}])?;", data.join(", ")));
                    }
                }
            }
        }
    }

    /// A value bound to a temporary unless simple.
    fn bind(&mut self, c: Code, out: &mut Vec<String>) -> String {
        out.extend(c.pre.iter().cloned());
        if c.simple && !c.m {
            return c.text;
        }
        let t = self.tmp();
        out.push(format!("let {t} = {};", c.text));
        t
    }

    fn keys(&mut self, place: &Place, out: &mut Vec<String>) -> Vec<String> {
        let mut keys = Vec::new();
        for step in &place.path {
            if let PlaceStep::MapKey(e) | PlaceStep::ListIndex(e) = step {
                let c = self.value(e);
                keys.push(self.bind(c, out));
            }
        }
        keys
    }

    fn root_text(&self, place: &Place) -> String {
        match &place.root {
            PlaceRoot::State(s) => format!("m.world.{}", self.cx.names.state(*s)),
            PlaceRoot::Local(n) => local(n),
        }
    }

    fn assign(&mut self, place: &Place, v: Code, out: &mut Vec<String>) {
        let trivial = place.check.as_ref().is_none_or(|d| self.cx.trivial(d));
        if place.path.is_empty() && trivial {
            out.extend(v.pre.iter().cloned());
            out.push(format!("{} = {};", self.root_text(place), v.text));
            return;
        }
        let value = self.bind(v, out);
        let keys = self.keys(place, out);
        if let Some(d) = &place.check
            && let Some(line) = self.check(d, &value, &format!("a write to {}", self.place_name(place)))
        {
            out.push(line);
        }
        out.push(self.write(place, &keys, &value));
    }

    fn place_name(&self, place: &Place) -> String {
        match &place.root {
            PlaceRoot::State(s) => self.cx.lir.state(*s).info.qualified.clone(),
            PlaceRoot::Local(n) => n.clone(),
        }
    }

    fn field_name(&self, ty: spec_lang_ir::TypeId, index: u32) -> String {
        match &self.cx.lir.ty(ty).def {
            TypeDef::Record { fields } => fields.get(index as usize).map_or("x".into(), |f| ident(&f.name)),
            _ => "x".into(),
        }
    }

    /// The statement writing `value` at the place, keys already evaluated.
    fn write(&self, place: &Place, keys: &[String], value: &str) -> String {
        let mut cur = self.root_text(place);
        let mut is_ref = false;
        let mut k = keys.iter();
        let n = place.path.len();
        for (i, step) in place.path.iter().enumerate() {
            let last = i + 1 == n;
            let amp = if is_ref { "" } else { "&mut " };
            match step {
                PlaceStep::Field { ty, index } => {
                    let f = self.field_name(*ty, *index);
                    if last {
                        let v = if self.cx.is_boxed(*ty, crate::cx::RECORD, *index) {
                            format!("Box::new({value})")
                        } else {
                            value.to_string()
                        };
                        return format!("{cur}.{f} = {v};");
                    }
                    cur = format!("{cur}.{f}");
                    is_ref = false;
                }
                PlaceStep::MapKey(_) => {
                    let key = k.next().cloned().unwrap_or_default();
                    if last {
                        return format!("{cur}.insert({key}, {value});");
                    }
                    cur = format!("rt::slot({amp}{cur}, &{key})?");
                    is_ref = true;
                }
                PlaceStep::ListIndex(_) => {
                    let key = k.next().cloned().unwrap_or_default();
                    if last {
                        return format!("*rt::item({amp}{cur}, {key})? = {value};");
                    }
                    cur = format!("rt::item({amp}{cur}, {key})?");
                    is_ref = true;
                }
            }
        }
        format!("{cur} = {value};")
    }

    /// The place's current value, keys already evaluated: a missing key or index fails `Missing`.
    fn read(&self, place: &Place, keys: &[String]) -> String {
        let mut cur = self.root_text(place);
        let mut is_place = true;
        let mut k = keys.iter();
        for step in &place.path {
            match step {
                PlaceStep::Field { ty, index } => cur = format!("{cur}.{}", self.field_name(*ty, *index)),
                PlaceStep::MapKey(_) => {
                    cur = format!("rt::list::map_at(&{cur}, &{})?", k.next().cloned().unwrap_or_default());
                    is_place = false;
                }
                PlaceStep::ListIndex(_) => {
                    cur = format!("rt::list::at(&{cur}, {})?", k.next().cloned().unwrap_or_default());
                    is_place = false;
                }
            }
        }
        if is_place && !self.cx.copy(&place.ty) { format!("{cur}.clone()") } else { cur }
    }

    fn mutate(&mut self, place: &Place, op: Prim, args: &[spec_lang_lir::node::Expr], out: &mut Vec<String>) {
        let mut vals = Vec::new();
        for a in args {
            let c = self.value(a);
            vals.push((self.bind(c, out), a.ty.clone()));
        }
        let trivial = place.check.as_ref().is_none_or(|d| self.cx.trivial(d));
        if place.path.is_empty() && trivial && vals.len() == 1 {
            let root = self.root_text(place);
            let a = &vals[0].0;
            let line = match op {
                Prim::ListPush => Some(format!("{root}.push({a});")),
                Prim::SetInsert => Some(format!("{root}.insert({a});")),
                Prim::SetRemove | Prim::MapRemove => Some(format!("{root}.remove(&{a});")),
                _ => None,
            };
            if let Some(line) = line {
                out.push(line);
                return;
            }
        }
        let keys = self.keys(place, out);
        let cur = self.tmp();
        out.push(format!("let {cur} = {};", self.read(place, &keys)));
        let mut items = vec![(Code { place: true, copy: self.cx.copy(&place.ty), ..Code::lit(cur) }, place.ty.clone())];
        items.extend(
            vals.into_iter().map(|(t, ty)| (Code { place: true, copy: self.cx.copy(&ty), ..Code::lit(t) }, ty)),
        );
        let new = self.prim_on(op, items, &place.ty);
        let v = self.tmp();
        out.extend(new.pre.iter().cloned());
        out.push(format!("let {v} = {};", new.text));
        if let Some(d) = &place.check
            && let Some(line) = self.check(d, &v, &format!("a write to {}", self.place_name(place)))
        {
            out.push(line);
        }
        out.push(self.write(place, &keys, &v));
    }
}

fn no_value() -> String {
    "return Err(rt::stop(\"E-RUNTIME\", \"the block ended without a value\"))".into()
}

/// Whether the text of a printed block touches `m` or can halt, read from the text itself.
fn guess(text: String, has_m: bool) -> Code {
    let m = has_m && (text.contains("m.") || text.contains("(m") || text.contains(" m,") || text.contains("m)"));
    let fail = text.contains('?') || text.contains("Err(");
    Code { m, fail, ..Code::val(text) }
}

/// A `return` inside an `if`, `for` or `while` of these statements (not inside a nested block expression).
fn nested_return(stmts: &[Stmt]) -> bool {
    fn inner(stmts: &[Stmt]) -> bool {
        stmts.iter().any(|s| match &s.kind {
            StmtKind::Return(_) => true,
            StmtKind::For { body, .. } | StmtKind::While { body, .. } => inner(&body.stmts),
            StmtKind::If { then, els, .. } => inner(&then.stmts) || els.as_ref().is_some_and(|b| inner(&b.stmts)),
            _ => false,
        })
    }
    stmts.iter().any(|s| match &s.kind {
        StmtKind::For { body, .. } | StmtKind::While { body, .. } => inner(&body.stmts),
        StmtKind::If { then, els, .. } => inner(&then.stmts) || els.as_ref().is_some_and(|b| inner(&b.stmts)),
        _ => false,
    })
}

/// Whether the statements never end normally.
pub fn diverges(stmts: &[Stmt]) -> bool {
    stmts.last().is_some_and(|s| match &s.kind {
        StmtKind::Return(_) => true,
        StmtKind::Expr(e) => matches!(e.ty, LTy::Never) || matches!(e.kind, ExprKind::Fail { .. } | ExprKind::Hole(_)),
        StmtKind::If { then, els: Some(b), .. } => diverges(&then.stmts) && diverges(&b.stmts),
        _ => false,
    })
}

/// Every local a statement list assigns or mutates, nested blocks and lambdas included.
pub fn assigned(stmts: &[Stmt], out: &mut BTreeSet<String>) {
    for s in stmts {
        match &s.kind {
            StmtKind::Assign { place, .. } | StmtKind::Mutate { place, .. } => {
                if let PlaceRoot::Local(n) = &place.root {
                    out.insert(n.clone());
                }
            }
            StmtKind::For { body, .. } | StmtKind::While { body, .. } => assigned(&body.stmts, out),
            StmtKind::If { then, els, .. } => {
                assigned(&then.stmts, out);
                if let Some(b) = els {
                    assigned(&b.stmts, out);
                }
            }
            _ => {}
        }
    }
    let wrap = Block {
        stmts: stmts.to_vec(),
        ty: LTy::Unit,
        span: spec_lang_syntax::Span::new(spec_lang_syntax::FileId(0), 0, 0),
    };
    let mut inner = Vec::new();
    walk::block_exprs(&wrap, &mut |e| match &e.kind {
        ExprKind::Block(b) => inner.push(b.stmts.clone()),
        ExprKind::Lambda(l) => {
            if let LambdaBody::Block(b) = &l.body {
                inner.push(b.stmts.clone());
            }
        }
        _ => {}
    });
    for b in inner {
        let mut sub = BTreeSet::new();
        assigned(&b, &mut sub);
        out.extend(sub);
    }
}

pub fn assigned_in_expr(e: &spec_lang_lir::node::Expr, out: &mut BTreeSet<String>) {
    let mut inner = Vec::new();
    walk::exprs(e, &mut |x| match &x.kind {
        ExprKind::Block(b) => inner.push(b.stmts.clone()),
        ExprKind::Lambda(l) => {
            if let LambdaBody::Block(b) = &l.body {
                inner.push(b.stmts.clone());
            }
        }
        _ => {}
    });
    for b in inner {
        assigned(&b, out);
    }
}

pub fn assigned_in_lambda(l: &Lambda) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    match &l.body {
        LambdaBody::Expr(x) => assigned_in_expr(x, &mut out),
        LambdaBody::Block(b) => assigned(&b.stmts, &mut out),
    }
    out
}

/// Every local name declared in an expression (lets, loop variables, lambda parameters, pattern bindings).
pub fn locals_in_expr(e: &spec_lang_lir::node::Expr, out: &mut BTreeSet<String>) {
    walk::exprs(e, &mut |x| match &x.kind {
        ExprKind::Local(n) => {
            out.insert(n.clone());
        }
        ExprKind::Lambda(l) => out.extend(l.params.iter().map(|(n, _)| n.clone())),
        _ => {}
    });
}
